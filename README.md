# rMetal
Librería de Optimización Metaheurística en Rust

<p align="center">
  <img src="./images/rMetalLogo1.png" alt="rMetal Logo" width="200"/>
</p>


## Descripción

Este proyecto consiste en una **biblioteca extensible de optimización metaheurística** desarrollada en Rust. Su objetivo es facilitar la implementación y experimentación con distintos algoritmos de optimización, proporcionando una arquitectura modular que permite añadir nuevos algoritmos, operadores y problemas sin modificar el núcleo del sistema.

Actualmente existen pocas librerías de este tipo en Rust y muchas presentan limitaciones en madurez o extensibilidad. Esta librería pretende ofrecer una alternativa flexible y de fácil uso para desarrolladores e investigadores.

---

## Características

- Arquitectura modular y extensible
- Implementación de varios algoritmos metaheurísticos
- Soporte para monitorización mediante observables
- Configuración de parámetros y experimentos
- Documentación y ejemplos de uso

---

## Instalación

1. Asegúrate de tener **Rust** y **Cargo** instalados.  
2. Añade la librería como dependencia en tu `Cargo.toml`:

```toml
[dependencies]
rMetal = "0.1.0"
