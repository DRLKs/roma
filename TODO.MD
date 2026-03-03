[X] 1. Refactorización de Traits y Genéricos (Sistema de Tipos)
    [X] Cambiar genéricos anidados (ej. `Algorithms<T, S, P>`) por Traits con Tipos Asociados (Associated Types). Ej: en el trait `Problem`, definir `type Variables;` y `type Fitness;`.
    [X] Aplanar la jerarquía de Traits. Dejar un único trait base `Algorithm` que defina el método de ejecución.

[X] 2. Estructura de la Solución (`Solution`)
    [X] Convertir `Solution` de un Trait a un Struct.
    [X] Eliminar las restricciones de traits (bounds como `<O: Ord>` o `Default`) en la definición del `struct Solution`, dejándolas solo en los bloques `impl` donde sea estrictamente necesario.

[X] 3. Adaptación a Optimización Multiobjetivo
    [X] Reemplazar `Set<f64>` por `Vec<f64>` para almacenar los objetivos (los f64 no implementan Eq ni Hash en Rust).
    [X] Eliminar la jerarquía orientada a objetos `QualityIndicator`.

[ ] 4. Operadores y Composición Estática
    [ ] Eliminar el uso de `Vec<Operator>` y `Box<dyn Operator>` (Dynamic Dispatch).
    [X] Inyectar los operadores concretos estáticamente mediante genéricos en el struct del algoritmo. Ej: `struct GeneticAlgorithm<P, S, C, M>`.

[ ] 5. Parámetros e Instanciación (Builders)
    [ ] Eliminar el struct monolítico `AlgorithmParameters`.
    [ ] Implementar el Patrón Builder para cada algoritmo específico. Esto permite configurar los parámetros particulares de cada uno (ej. `population_size`, `mutation_rate`) con validación en tiempo de compilación.

[X] 6. Manejo de Eventos y Observadores
    [X] Eliminar el patrón Observer clásico (`Observable`, `AlgorithmObserver`) que obliga a manejar lifetimes o punteros compartidos complejos.
    [X] Reemplazar la notificación de progreso por el uso de Closures (`FnMut`) pasados al Builder, o canales de paso de mensajes (`std::sync::mpsc`).
    [ ] Hemos añadido más información. Cuando el algoritmo termina, termina debido a una condición de parada, estoa condición de parada debe mostrarse en los charts o en lo repots o en ambos. Terminal, etc...

[ ] 7. Memoria, requisitos
    [ ] Desarrollar los casos de uso. Crear tus propios algoritmos y ejemplos o añadir el crate y usar los algoritmos de la biblioteca.
    [ ] .....

[X] 8. Reproducibilidad 28/02/26

[ ] 9. Algoritmos y operadores, ideas de Gabriel
    [ ] Más criterios de paradas.
    [ ] Operadores, que devuelvan información para quizás hacer más rápidas las evaluaciones de las soluciones, porque devolverán información sobre lo que ha cambiado de la solución

[X] 10. Ideas, Observadores y Criterios de paradas.  Practicamente 03/03/2026
    [X] Ambas estructuras almacenan información sobre el estado actual del algoritmo, quizás juntar esta información en una estructura más grande

[ ] 11. CSVs, JSON, YAML. Quiero que también se pueda añadir datos desde distintos tipos de archivos. Solo está implementado para CSVs, que de hecho es el que peor funciona para estos casos.

[ ] 12. Creo que deberiamos quitar el evento BestSolutionEvent, con ExecutionSOlutionUpdate va perfe. Creo que no hace falta que añadamos más lógica al run.
