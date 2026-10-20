# Cabina y modelo de vuelo del F-16

Revisión del 20 de octubre de 2026. La base elegida es el F-16C Block 50 con
motor F110-GE-129 y pantallas multifunción en color. El modelado es procedural,
con un acabado de juego de pocos polígonos. Las fotografías se usan como
referencia visual y no se incluyen como texturas ni se redistribuyen.

## Referencias visuales

- [Galería de cabinas y HUD de f-16.net](https://www.f-16.net/g3/f-16-photos/album03/album72).
  Distribución general, proporciones, consolas y disposición de los controles.
- [Vista frontal de F-16C/D Block 40, fotografía USAF](https://www.f-16.net/g3/f-16-photos/album03/album72/aaa).
  Sirve para estudiar la relación entre instrumentos, piernas y manos. Es un
  Block 40; su HUD específico no se considera una referencia exacta del Block 50.
- [Palanca de potencia, Martin Ingold](https://www.f-16.net/g3/f-16-photos/album03/album72/aak).
  Control izquierdo, empuñadura y consola.
- [Palanca lateral, Martin Ingold](https://www.f-16.net/g3/f-16-photos/album03/album72/aal).
  Empuñadura derecha y apoyo del brazo.
- [HUD de F-16C, fotografía USAF](https://www.f-16.net/g3/f-16-photos/album03/album72/aad).
  Color verde, transparencia y relación entre símbolos y paisaje. La fotografía
  muestra el HUD gran angular del Block 40/42; el juego usa una adaptación
  compacta, no una reproducción de ese equipo en un Block 50.
- [Descripción del Block 50/52 en f-16.net](https://www.f-16.net/f-16_versions_article9.html).
  Variante de motor, dos MFD y modernización con pantallas en color.

La [ficha de la USAF sobre el F-16C](https://www.jba.af.mil/About/Fact-Sheets/Display/Article/4282145/f-16c-fighting-falcon/)
confirma el asiento inclinado 30 grados, la palanca lateral sensible a presión,
la visibilidad de la cúpula y la capacidad de 9 G. La cámara se sitúa detrás de
las caderas y sobre el torso, dentro de la cúpula exterior. Las coordenadas de
la cabina son aproximadas; no proceden de un escaneo o plano dimensional.

## Correspondencia con la simulación

| Instrumento | Dato o comportamiento |
| --- | --- |
| HUD y anemómetro | Velocidad calibrada en nudos, calculada con presión de impacto y atmósfera ISA |
| Mach | Velocidad verdadera dividida por la velocidad local del sonido |
| Altímetro y B en HUD | Altitud sobre el nivel medio del mar, en pies |
| R en HUD | Distancia vertical al terreno bajo el centro del avión, en pies |
| ADI y escala de cabeceo | Actitud del cuaternión y horizonte inercial |
| FPM | Dirección real del vector de velocidad, proyectada desde la cámara |
| HSI y rumbo | Rumbo verdadero del modelo; no se modela declinación magnética |
| G y máximo G | Fuerza normal específica y máximo registrado en la salida |
| AOA | Ángulo entre flujo y eje longitudinal |
| HSD | Ruta real, rumbo del avión y siguiente puerta como steerpoint |
| DED | Radios fijas de ambientación, steerpoint, rumbo, distancia, tiempo y combustible |
| RPM, FTIT, tobera y aceite | Estimaciones a partir del régimen del motor y la postcombustión |
| Combustible y caudal | Masa consumida por el modelo; conversión a libras y libras por hora |
| Alarmas | Combustible bajo, motor sin combustible, AOA, G, exceso de Mach y proximidad al terreno |
| Manos y pedales | Entradas de control suavizadas; la mano izquierda acompaña el recorrido de potencia |

El HUD corresponde al modo NAV del juego. No se simulan modos de armamento,
ILS, navegación INS con errores, TACAN, comunicaciones, contactos radar ni
emisiones enemigas. FCR muestra una exploración sin contactos y RWR muestra
ausencia de emisores. Los pulsadores del ICP y conmutadores de consola son
decorativos; las páginas de los MFD se cambian con F1 y F2. La lectura AGL no
incluye el haz, los límites de actitud o las averías de un radioaltímetro real.

## Parámetros y calibración

La ficha del Block 50/52 de f-16.net proporciona empuje seco de 17.155 lbf y
postcombustión de 28.984 lbf para el F110-GE-129, superficie alar de 300 pies
cuadrados, masa vacía de 18.238 lb y velocidad máxima de corta duración de
Mach 2,05 a 40.000 pies. En el código se usan aproximadamente 76.300 N,
128.922 N, 27,87 metros cuadrados y 8.273 kg, respectivamente.

Se mantienen masa variable con combustible, sustentación dependiente de
presión dinámica y AOA, resistencia inducida, resistencia transónica, efecto
suelo, pérdida, límites de mando, retardo del motor y reducción del empuje con
altura. Los coeficientes y curvas son ajustes de juego; no se han extraído las
tablas aerodinámicas completas de una variante real. El límite de AOA de unos
26 grados y los consumos son aproximaciones de esta implementación.

`node tests/performance.mjs` calcula equilibrio ideal de fuerzas a 1 G y plena
postcombustión, con el peso inicial de la misión. Resultado de esta revisión:

| Altitud | Mach de equilibrio del modelo |
| --- | --- |
| 10.000 pies | 1,653 |
| 20.000 pies | 1,740 |
| 30.000 pies | 1,869 |
| 40.000 pies | 2,019 |

La coincidencia aproximada con Mach 2,05 a 40.000 pies es una comprobación de
escala, no una validación de toda la envolvente. No se modelan límites térmicos,
restricciones por cargas externas, protección completa FLCS, daños progresivos,
viento ni tablas medidas de motor y aerodinámica. No se limita artificialmente
la velocidad a ese valor: las fuerzas determinan la aceleración y el HUD avisa
cuando se supera Mach 2,05.

## Verificación

- `tests/physics.mjs`: ISA, velocidad calibrada subsónica y supersónica,
  diferencias por velocidad y altitud, combustible agotado, retardo de
  postcombustión, vuelo invertido, aerofreno, instrumentos y estabilidad numérica.
- `tests/browser.html`: arranque WebGL, separación entre vista libre y mandos,
  páginas MFD, manos, pausa, reanudación, vista externa, diez puertas, victoria,
  tiempo agotado, colisión y reinicio.
- Inspección visual de frente, ambos laterales, piloto y cola. Alt más mouse
  permite girar la cabeza hasta 155 grados por lado y 85 grados en vertical.
  C devuelve la vista al frente. La cabina y el exterior comparten posición,
  escala y actitud; la cúpula opaca del modelo externo se omite en primera persona.
