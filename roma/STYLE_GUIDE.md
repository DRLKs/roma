## Guía de Estilo y Patrones — Proyecto Rust


| Elemento | Convención | Ejemplo | Notas |
|-----------|-------------|----------|-------|
| **Structs** | `PascalCase` | `UserProfile`, `ConfigFile` | Representan datos o entidades. |
| **Campos de struct** | `snake_case` | `user_id`, `created_at` | Siempre en minúsculas con guiones bajos. |
| **Traits** | `PascalCase` | `Display`, `Runnable`, `Serializable` | Normalmente adjetivos o verbos. |
| **Métodos de traits** | `snake_case` | `fn run(&self)` | Describen acciones. |
| **Funciones** | `snake_case` | `fn calculate_total()` | Nombres descriptivos y en minúsculas. |
| **Variables** | `snake_case` | `let total_price = 0.0;` | Claras y expresivas. |
| **Constantes** | `SCREAMING_SNAKE_CASE` | `const MAX_SIZE: usize = 100;` | Usar prefijos si aplica (`APP_`, `DB_`, etc.). |
| **Módulos** | `snake_case` | `mod data_parser;` | Coinciden con el nombre del archivo. |
| **Enums** | `PascalCase` | `enum Status { Active, Inactive }` | Variantes también en PascalCase. |
| **Archivos** | `snake_case.rs` | `user_profile.rs` | Un archivo por módulo o tipo principal. |
| **Constructor principal** | `new()` | `User::new(name, email)` | Para crear instancias básicas. |
| **Constructores alternativos** | `from_` / `with_` | `Config::from_file()`, `User::with_admin()` | Usar prefijos descriptivos. |
| **Builder pattern** | `StructBuilder` | `UserBuilder`, `ConfigBuilder` | Para objetos con parámetros opcionales. |
| **Método de acceso al builder** | `.builder()` | `User::builder()` | Devuelve una instancia del builder. |
| **Método final del builder** | `.build()` | `user.build()` | Devuelve el objeto final. |
| **Factory pattern** | `StructFactory` | `ShapeFactory`, `RepoFactory` | Para crear diferentes implementaciones de un trait. |
| **Función Factory** | `create()` / `create_xxx()` | `ShapeFactory::create("circle")` | Devuelve `Box<dyn Trait>` u objeto concreto. |
| **Nombres de tests** | `snake_case` | `#[test] fn creates_valid_user()` | Descriptivos, con guiones bajos. |
| **Comentarios de documentación** | `///` triple barra | `/// Crea un nuevo usuario.` | Usar estilo docstring Markdown. |
| **Errores personalizados** | `PascalCase + Error` | `ConfigError`, `ParseError` | Implementar `std::error::Error`. |

---

**Resumen de patrones recomendados**

| Caso | Patrón recomendado | Ejemplo |
|------|--------------------|----------|
| Objeto simple con pocos parámetros | `new()` o `from_()` | `User::new()`, `Config::from_file()` |
| Objeto con muchos parámetros opcionales | **Builder** | `User::builder().email(...).build()` |
| Crear distintas implementaciones de un trait | **Factory** | `ShapeFactory::create("circle")` |
| Transformación de un tipo a otro | `From` / `Into` traits | `impl From<&str> for User` |

---
