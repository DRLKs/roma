TODO: Refactorización de Arquitectura de Biblioteca Metaheurística en Rust

[ ] 1. Refactorización de Traits y Genéricos (Sistema de Tipos)
    [X] Cambiar genéricos anidados (ej. `Algorithms<T, S, P>`) por Traits con Tipos Asociados (Associated Types). Ej: en el trait `Problem`, definir `type Variables;` y `type Fitness;`.
    [ ] Aplanar la jerarquía de Traits. Dejar un único trait base `Algorithm` que defina el método de ejecución.
    [ ] Implementar las variaciones de algoritmos usando structs concretos (ej. `struct GeneticAlgorithm`) que implementen el trait `Algorithm`, evitando la herencia ("composición sobre herencia").

[ ] 2. Estructura de la Solución (`Solution`)
    [X] Convertir `Solution` de un Trait a un Struct.
    [X] Eliminar las restricciones de traits (bounds como `<O: Ord>` o `Default`) en la definición del `struct Solution`, dejándolas solo en los bloques `impl` donde sea estrictamente necesario.

[ ] 3. Adaptación a Optimización Multiobjetivo
    [X] Reemplazar `Set<f64>` por `Vec<f64>` para almacenar los objetivos (los f64 no implementan Eq ni Hash en Rust).
    [ ] Eliminar la jerarquía orientada a objetos `QualityIndicator`.
    [ ] Crear un trait `Dominance` para encapsular la lógica de dominancia de Pareto y comparar los `Vec<f64>`.
    [ ] Utilizar el parámetro genérico de atributos `A` en la `Solution` para inyectar metadatos multiobjetivo como "Rango" (Rank) y "Distancia de Apelotonamiento" (Crowding Distance) solo cuando el algoritmo lo requiera.
    [ ] Evaluar si mantener `SolutionSet`; cambiarlo a un `Vec<Solution>` normal o refactorizarlo hacia un concepto más específico como `ParetoArchive` (Non-dominated Set).

[ ] 4. Operadores y Composición Estática
    [ ] Eliminar el uso de `Vec<Operator>` y `Box<dyn Operator>` (Dynamic Dispatch).
    [ ] Inyectar los operadores concretos estáticamente mediante genéricos en el struct del algoritmo. Ej: `struct GeneticAlgorithm<P, S, C, M>`.

[ ] 5. Parámetros e Instanciación (Builders)
    [ ] Eliminar el struct monolítico `AlgorithmParameters`.
    [ ] Implementar el Patrón Builder para cada algoritmo específico. Esto permite configurar los parámetros particulares de cada uno (ej. `population_size`, `mutation_rate`) con validación en tiempo de compilación.

[ ] 6. Manejo de Eventos y Observadores
    [ ] Eliminar el patrón Observer clásico (`Observable`, `AlgorithmObserver`) que obliga a manejar lifetimes o punteros compartidos complejos.
    [ ] Reemplazar la notificación de progreso por el uso de Closures (`FnMut`) pasados al Builder, o canales de paso de mensajes (`std::sync::mpsc`).

[ ] 7. Manejo de Errores Idiomático
    [ ] Eliminar el evento `Error` del antiguo enum `AlgorithmEvent`.
    [ ] Refactorizar el trait principal para que el método devuelva un resultado estándar de Rust: `fn run(&mut self) -> Result<Self::SolutionSet, Self::Error>;`.
