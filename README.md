<img src="./images/rMetal_logo.png" alt="rMetal Logo" width="250"/>


<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Made%20with-Rust-black?style=for-the-badge&logo=rust" alt="Made with Rust"></a>
  <a href="https://crates.io/"><img src="https://img.shields.io/badge/crates.io-soon-red?style=for-the-badge&logo=rust" alt="Crates.io"></a>
  <a href="https://docs.rs/"><img src="https://img.shields.io/badge/docs.rs-WIP-blue?style=for-the-badge&logo=rust" alt="Docs.rs"></a>
  <img src="https://img.shields.io/badge/Status-Development-yellow?style=for-the-badge" alt="Build Status">
</p>

---

## Descripción y Motivación

`rMetal` es una biblioteca de optimización metaheurística escrita íntegramente en Rust. Su objetivo principal es proporcionar un marco de trabajo (*framework*) potente, flexible e idiomático para resolver problemas de optimización complejos, tanto de un solo objetivo (mono-objetivo) como de múltiples objetivos (multiobjetivo).

La motivación detrás de `rMetal` surge de la necesidad de aplicar la **seguridad de memoria y el alto rendimiento** de Rust al campo de la investigación operativa.

Este proyecto es el resultado de mi **TFG**, con el que termino mis estudios sobre la ingeniería del software.

---

## Instalación

`rMetal` está diseñado para ser integrado en proyectos Rust existentes. Una vez publicado en crates.io (actualmente en WIP), podrás añadirlo a tu `Cargo.toml`:

```toml
[dependencies]
rmetal = "0.1.0"
# O directamente desde git mientras está en desarrollo
# rMetal = { git = "[https://github.com/DRLKs/rMetal.git](https://github.com/DRLKs/rMetal.git)" }