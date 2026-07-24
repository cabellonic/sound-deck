//! Validacion de URLs remotas (§30).
//!
//! Aunque esto sea una aplicacion de escritorio, tratamos toda URL que venga de
//! un proveedor como no confiable: solo HTTPS, solo hosts declarados por el
//! propio proveedor, y nunca direcciones internas.

use crate::errors::{AppError, AppResult, ErrorKind};

/// Esquemas permitidos para descargar o previsualizar audio.
const ALLOWED_SCHEMES: [&str; 2] = ["https", "http"];

/// Parte relevante de una URL, extraida sin dependencias externas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
}

/// Descompone una URL absoluta en esquema, host y puerto.
pub fn parse_url(raw: &str) -> Option<ParsedUrl> {
    let (scheme, rest) = raw.split_once("://")?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }

    // El autoridad termina en el primer `/`, `?` o `#`.
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        return None;
    }

    // Descartamos userinfo (`user:pass@host`): no aceptamos credenciales embebidas.
    if authority.contains('@') {
        return None;
    }

    let (host, port) = match authority.rsplit_once(':') {
        // IPv6 literal: `[::1]:8080`. Sin corchetes de cierre no es un puerto.
        Some((host, port)) if !host.ends_with(']') || host.starts_with('[') => {
            match port.parse::<u16>() {
                Ok(port) => (host, Some(port)),
                Err(_) => (authority, None),
            }
        }
        _ => (authority, None),
    };

    if host.is_empty() {
        return None;
    }

    Some(ParsedUrl {
        scheme: scheme.to_ascii_lowercase(),
        host: host.trim_matches(['[', ']']).to_ascii_lowercase(),
        port,
    })
}

/// Hosts que nunca aceptamos: apuntan a la propia maquina o a la red local.
/// Es la mitigacion basica de SSRF pedida por §30.
fn is_internal_host(host: &str) -> bool {
    if matches!(host, "localhost" | "127.0.0.1" | "0.0.0.0" | "::1" | "::") {
        return true;
    }
    if host.ends_with(".localhost") || host.ends_with(".local") || host.ends_with(".internal") {
        return true;
    }

    // Rangos privados IPv4.
    let octets: Vec<&str> = host.split('.').collect();
    if octets.len() == 4 {
        if let (Ok(a), Ok(b)) = (octets[0].parse::<u8>(), octets[1].parse::<u8>()) {
            let numeric = octets.iter().all(|part| part.parse::<u8>().is_ok());
            if numeric {
                return a == 10
                    || a == 127
                    || (a == 192 && b == 168)
                    || (a == 172 && (16..=31).contains(&b))
                    || (a == 169 && b == 254);
            }
        }
    }

    // Direcciones IPv6 unique-local y link-local.
    host.starts_with("fc") || host.starts_with("fd") || host.starts_with("fe80")
}

/// Verifica que la URL sea descargable y pertenezca a un host declarado por el
/// proveedor. `allowed_hosts` acepta el dominio exacto o cualquier subdominio.
pub fn validate_remote_url(raw: &str, allowed_hosts: &[&str]) -> AppResult<ParsedUrl> {
    let parsed = parse_url(raw).ok_or_else(|| {
        AppError::new(
            ErrorKind::Validation,
            "El proveedor devolvio una direccion de descarga que no es valida.",
        )
        .with_technical(format!("url no parseable: {raw}"))
    })?;

    if !ALLOWED_SCHEMES.contains(&parsed.scheme.as_str()) {
        return Err(AppError::new(
            ErrorKind::Validation,
            "Solo se aceptan descargas por HTTP o HTTPS.",
        )
        .with_technical(format!("esquema rechazado: {}", parsed.scheme)));
    }

    if is_internal_host(&parsed.host) {
        return Err(AppError::new(
            ErrorKind::Validation,
            "La direccion de descarga apunta a la red local y fue rechazada.",
        )
        .with_technical(format!("host interno: {}", parsed.host)));
    }

    let allowed = allowed_hosts.iter().any(|candidate| {
        let candidate = candidate.to_ascii_lowercase();
        parsed.host == candidate || parsed.host.ends_with(&format!(".{candidate}"))
    });

    if !allowed {
        return Err(AppError::new(
            ErrorKind::Validation,
            "La direccion de descarga no pertenece al proveedor elegido.",
        )
        .with_technical(format!("host {} fuera de {allowed_hosts:?}", parsed.host)));
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTS: [&str; 2] = ["freesound.org", "cdn.freesound.org"];

    #[test]
    fn parsea_urls_normales() {
        let parsed = parse_url("https://cdn.freesound.org/previews/1/2-hq.mp3").unwrap();
        assert_eq!(parsed.scheme, "https");
        assert_eq!(parsed.host, "cdn.freesound.org");
        assert_eq!(parsed.port, None);

        let con_puerto = parse_url("http://ejemplo.com:8080/a.mp3").unwrap();
        assert_eq!(con_puerto.port, Some(8080));
    }

    #[test]
    fn rechaza_urls_malformadas() {
        assert!(parse_url("no-es-una-url").is_none());
        assert!(parse_url("https://").is_none());
        assert!(parse_url("://sinhost").is_none());
        // Credenciales embebidas: no.
        assert!(parse_url("https://user:pass@ejemplo.com/a.mp3").is_none());
    }

    #[test]
    fn acepta_el_host_del_proveedor_y_sus_subdominios() {
        assert!(validate_remote_url("https://freesound.org/a.mp3", &HOSTS).is_ok());
        assert!(validate_remote_url("https://cdn.freesound.org/x/y.mp3", &HOSTS).is_ok());
        assert!(validate_remote_url("https://media.cdn.freesound.org/x.mp3", &HOSTS).is_ok());
    }

    #[test]
    fn rechaza_hosts_ajenos_al_proveedor() {
        let error = validate_remote_url("https://malicioso.com/a.mp3", &HOSTS).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Validation);
        // Un sufijo parecido no alcanza para colarse.
        assert!(validate_remote_url("https://notfreesound.org/a.mp3", &HOSTS).is_err());
        assert!(validate_remote_url("https://freesound.org.evil.com/a.mp3", &HOSTS).is_err());
    }

    #[test]
    fn rechaza_esquemas_peligrosos() {
        for url in [
            "file:///C:/Windows/System32/config/sam",
            "javascript://freesound.org/alert(1)",
            "data://freesound.org/audio",
        ] {
            assert!(validate_remote_url(url, &HOSTS).is_err(), "{url}");
        }
    }

    #[test]
    fn rechaza_destinos_internos_aunque_el_host_coincida() {
        for host in [
            "localhost",
            "127.0.0.1",
            "10.0.0.5",
            "192.168.1.10",
            "172.16.0.1",
            "169.254.169.254",
            "algo.local",
        ] {
            assert!(is_internal_host(host), "{host} deberia ser interno");
            assert!(
                validate_remote_url(&format!("https://{host}/a.mp3"), &[host]).is_err(),
                "{host}"
            );
        }
    }

    #[test]
    fn una_ip_publica_no_se_confunde_con_una_privada() {
        assert!(!is_internal_host("172.32.0.1"));
        assert!(!is_internal_host("11.0.0.1"));
        assert!(!is_internal_host("freesound.org"));
    }
}
