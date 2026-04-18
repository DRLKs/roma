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

[ ] 6. Manejo de Eventos y Observadores
    [X] Eliminar el patrón Observer clásico (`Observable`, `AlgorithmObserver`) que obliga a manejar lifetimes o punteros compartidos complejos.
    [X] Reemplazar la notificación de progreso por el uso de Closures (`FnMut`) pasados al Builder, o canales de paso de mensajes (`std::sync::mpsc`).
    [ ] Hemos añadido más información. Cuando el algoritmo termina, termina debido a una condición de parada, esta condición de parada debe mostrarse en los charts o en lo repots o en ambos. Terminal, etc...

    [ ] Arreglar los observadores. Mejorar los HTML report observers

[X] 7. Memoria, requisitos
    [X] Desarrollar los casos de uso. Crear tus propios algoritmos y ejemplos o añadir el crate y usar los algoritmos de la biblioteca.
    [X] .....

[X] 8. Reproducibilidad 28/02/26

[ ] 9. Algoritmos y operadores, ideas de Tutor
    [X] Más criterios de paradas.
    [ ] Operadores, que devuelvan información para quizás hacer más rápidas las evaluaciones de las soluciones, porque devolverán información sobre lo que ha cambiado de la solución

[X] 10. Ideas, Observadores y Criterios de paradas. MUY CORE.  - - - Asignado para --> 09/03/2026 u 10/03/2026
    [X] Ambas estructuras almacenan información sobre el estado actual del algoritmo, quizás juntar esta información en una estructura más grande    
    ---Algoritmos
    [ ] Quizás quitar get_solution_set y get_solution_set
    [X] Mi idea general es que la función run, sea la misma para todos los algoritmos. 
        Hilo distinto para que los observadores estén en otro hilo {Validate parameters, while !should_terminate : step(), context.update }.
        Luego step cambia para cada algoritmo.
    [X] No me gustan los panics, quizás quitarlos y cambiarlos por Result<>
    [ ] Quizás si no tuvieramos que copiar los criterios de terminación estaría bastante bien
    [X] Cambiar validate_parameters para que devuelva un result o algo así con el mensaje de error en caso de que los parámetros estén mal

[ ] 11. CSVs, JSON, YAML. Quiero que también se pueda añadir datos desde distintos tipos de archivos. Solo está implementado para CSVs, que de hecho es el que peor funciona para estos casos.
    [X] Mejorar esta implementación, por ahora muy mala (Más o menos)
    [ ] Tiene margen de mejora

[ ] 12. Reestructurar el proyecto
    [X] ExecutionStateSnapshot no creo que deba estar en Termination.
    [ ] La estructura que manejamos de traits en cada módulo la vi de un repositorio de X(Twitter), pero no se si será lo mejor para esta librería.
 
[X] 13. Aunque sea duro decirlo, creo que debemos volver a mirar Solution. Para el multiobjetivo va bastante raro. Obtener la mejor solución es más aleatorio en estos casos que algo real. Se observa el primer objetivo, que quizás si estamos mirando Pareto tiene sentido, pero lo dudo enormemente

[ ] 14. Para el tema de los experimentos, debemos regular de alguna manera los parámetros de los algoritmos. Debe existir alguna manera de categorizar los parámetros para que sean más intuitivo y sencilla la creación de ejecuciones de pruebas y experimentos. Quizás que los parámetros implementen un trait o alguna otra cosa, supongo que existirá algo en Rust para solucionarlo.
    [ ] Añadir más formas para comparar las ejecuciones.
    [X] Crear observadores especiales para experimentos.
    [X] Quizás directamente que los algoritmos puedan implementar algo para que puedan probarse en experimentos. **

    ( lo implementado a mi parecer es muy verboso, debemos observarlo con cautela y modificarlo si fuera necesario )
    ( los observadores de los experimentos creo que están bastante decentes, pero seguramente sean mejorables )

[ ] 15. Creación de macros para hacer más abstracta la biblioteca, esto quizás es demasiado avanzado

[X] 16. Experimentos.
    [X] Estandarizar los parámetros para que se puedan añadir a los summaries { Hecho a medias con el parser, es mejorable }
    [X] El algoritmo debe recibir parámetros, estos parámetros construir algoritmos y que estos se ejecuten

[X] 17. TAREA 12/03/2026: Pruebas + Limpiar código + Arquitectura
[ ] 18. Limpiar mucho el código 
[ ] 19. Los problemas multiobjetivo, son inresolvibles por algoritmo de mono-objetivo, quizás deberiamos hacer reestricciones para que estos algoritmos no puedan solucionarlos
[X] 20. Sin duda existen redundancias en la elección de que si el modelo es de minimización o de optimización. 
    [X]Quizás el compare solutions debería estar en problema, porque que una solución sea mejor que otra creo que depende más del problema que de la misma solución.

[ ] 21. Hacer representaciones de las soluciones dependientemente del problema

[ ] 22. Desarrollar reportes sobre los experimentos



Cambiar algorithm.run añadir el argumento resume