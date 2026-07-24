// Evita que se abra una consola junto a la ventana en Windows release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sound_deck_lib::run();
}
