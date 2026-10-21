# Cabina y modelo de vuelo del F-16

Revisión del 20 de octubre de 2026, versión nativa en Rust. La base de vuelo
sigue siendo el F-16C Block 50 con motor F110-GE-129, portada línea a línea
desde la versión web. La cabina sigue la imagen modernizada del usuario, con dos
pantallas laterales verticales y una pantalla central CPD, como en la
modernización F-16V, pero ahora es geometría 3D real a distancias realistas del
ojo de diseño. El modelado es procedural, con superficies suaves y acabado retro
mediante resolución, color y texturas. Las fotografías se usan como referencia
visual y no se incluyen como texturas ni se redistribuyen.

## Referencias visuales

- Las cuatro imágenes proporcionadas por el usuario en esta conversación:
  disposición de la cabina modernizada y vistas superior, frontal y lateral
  del avión. No se convierten en texturas ni se incluyen como archivos del juego.
- [Descripción del F-16V por Lockheed Martin](https://lockheedmartin.com/en-us/news/features/2016/meet-the-f-16v--the-most-technologically-advanced-4th-generation.html).
  Confirma la pantalla central CPD de 6 por 8 pulgadas en la modernización.
- [Galería de cabinas y HUD de f-16.net](https://www.f-16.net/g3/f-16-photos/album03/album72).
  Distribución general, proporciones, consolas y disposición de los controles.
- [Vista frontal de F-16C/D Block 40, fotografía USAF](https://www.f-16.net/g3/f-16-photos/album03/album72/aaa).
  Sirve para estudiar la relación entre instrumentos, piernas y manos. Es un
  Block 40; su HUD específico no se considera una referencia exacta del Block 50.
- [Palanca de potencia, Martin Ingold](https://www.f-16.net/g3/f-16-photos/album03/album72/aak).
  Control izquierdo, empuñadura y consola.
- [Diagrama de la empuñadura del throttle, Wikimedia Commons](https://commons.wikimedia.org/wiki/File:F-16_Throttle.jpg)
  y [réplica 1:1 Thrustmaster Viper TQS](https://shop.thrustmaster.com/en_gb/viper-tqs.html).
  Fijan la forma del throttle del juego: una palanca en el borde exterior de
  la consola izquierda lleva una empuñadura que sale hacia dentro, subiendo
  unos 30 grados, desde un bulbo grueso junto a la palanca, por una cintura,
  hasta la cabeza de mandos en el extremo interior. Arriba de la cabeza van la
  perilla MAN RNG/UNCAGE, DOG FIGHT y el interruptor de comunicaciones; en el
  extremo, la rueda ANT ELEV; en la cara trasera, el cursor del radar y el
  aerofreno. La palanca de corte queda por fuera. La mano izquierda la toma en
  horizontal desde arriba: dorso arriba, dedos por delante y por debajo y el
  pulgar por la cara trasera hacia el cursor.
- [Palanca lateral, Martin Ingold](https://www.f-16.net/g3/f-16-photos/album03/album72/aal).
  Empuñadura derecha y apoyo del brazo.
- [HUD de F-16C, fotografía USAF](https://www.f-16.net/g3/f-16-photos/album03/album72/aad).
  Color verde, transparencia y relación entre símbolos y paisaje. La fotografía
  muestra el HUD gran angular del Block 40/42; el juego usa una adaptación
  compacta, no una reproducción de ese equipo en un Block 50.
- [Descripción del Block 50/52 en f-16.net](https://www.f-16.net/f-16_versions_article9.html).
  Variante de motor y parámetros de vuelo de referencia. La disposición del
  panel actual se inspira en la modernización posterior, no en el panel básico de ese Block.

La [ficha de la USAF sobre el F-16C](https://www.jba.af.mil/About/Fact-Sheets/Display/Article/4282145/f-16c-fighting-falcon/)
confirma el asiento inclinado 30 grados, la palanca lateral sensible a presión,
la visibilidad de la cúpula y la capacidad de 9 G. El ojo de diseño está en
(0; 1,25; -3,24) m en ejes del avión, detrás de las caderas por la inclinación
del asiento y dentro de la cúpula. El panel está a unos 0,9 m del ojo, el
cristal combinador del HUD a unos 0,75 m y el UFC (DED sobre el teclado ICP)
justo debajo del HUD. Las dos MFD verticales flanquean la CPD central; a la
izquierda quedan el RWR, el anemómetro y el altímetro de reserva, y a la
derecha el ADI de reserva y los indicadores redondos de RPM, FTIT, tobera y
aceite, cada uno con su bisel circular. La cúpula es una única capa convexa de
cristal con tinte leve, brillo de Fresnel y reflejos del sol, de modo que cada
píxel la atraviesa una sola vez. Las coordenadas son aproximadas; no proceden
de un escaneo o plano dimensional.

El FOV predeterminado baja de 78 a 58 grados, con ajuste entre 45 y 72. La
sensibilidad predeterminada del mouse es 2,8 veces la original. K abre el panel
izquierdo y pausa el vuelo mientras se configura; H saca o recoge el
aerofreno. El cabeceo sale de una sola entrada, elegida en ese panel: el mouse
(por defecto) o W/S; la otra queda inactiva. La inversión se aplica al eje
vertical del mouse; S tira y W empuja siempre. El eje X del mouse mueve el
timón y vuelve solo al centro.

La ley de cabeceo del FLCS manda factor de carga: además de la velocidad de
cabeceo que ese factor produce, añade un término de ángulo de ataque que
construye la sustentación en unos 0,3 s, en vez de esperar la respuesta propia
del avión, de alrededor de 1 s. Con tirón completo a la velocidad inicial se
llega a unos 6 G en 0,4 s y a 9 G en menos de un segundo; al soltar, vuelve a
1 G en medio segundo. Con la palanca centrada el estado estacionario es el
mismo mantenimiento de trayectoria de antes, y el mando no lleva el avión por
sí solo más allá de unos 16 grados de AOA.

El exterior es un fuselaje de secciones superelípticas con radomo ojival, toma
ventral en D con su separador de capa límite, LERX fundidos con el ala, ala de
40 grados de flecha con perfil NACA del 4 %, deriva de unos 47 grados con
alojamiento del paracaídas, estabilizadores monobloque con 10 grados de
anhedro, aletas ventrales inclinadas, tobera de pétalos y cúpula de burbuja con
tinte dorado. Estabilizadores, timón y aerofrenos se mueven con los mandos. Las
pruebas miden la malla real: 15,06 metros de longitud, 9,96 metros de
envergadura sobre los AIM-9 de punta de ala y 5,09 metros de altura desde la
línea del tren. Los diez contactos de colisión (morro, cúpula, cola, puntas de
ala, toma, vientre, deriva y puntas de los estabilizadores) siguen esa malla.
La cabina sigue siendo una adaptación visual para juego.

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

### Modelo de vuelo

El modelo sigue al F-16 de FlightGear (repositorio NikolaiVChr/f16, JSBSim):

- Aerodinámica: tablas de NASA TP-1538 (Nguyen et al., 1979) en la forma
  reducida de Stevens y Lewis, *Aircraft Control and Simulation*: CX, CZ y Cm
  frente a AOA y estabilizador; Cl y Cn frente a AOA y derrape; efectos de
  flaperón y timón; derivadas de amortiguamiento. Es el mismo conjunto de datos
  que JSBSim usa en forma completa. Las tablas se comprobaron contra la
  transcripción pública de AeroBenchVV.
- De FlightGear (`Systems/jsb-aerodynamics-block15-subtle.xml`): resistencia de
  onda frente a Mach, desplazamiento transónico del centro aerodinámico
  (estimación de Nikolai V. Chr.) y efecto suelo.
- Masa e inercias del Block 50 (`Systems/jsb-mass-50.xml`): 19.200 lb vacío,
  Ixx 9.007, Iyy 49.956, Izz 56.770 e Ixz 198 slug·ft²; se añaden piloto,
  misiles de punta de ala y combustible (7.000 lb internas), que se consume.
- Motor F110-GE-129 (`Engines/F110-GE-129.xml`): 17.000 lbf militar y 29.000
  lbf con postcombustión, tablas de empuje frente a Mach y altitud, TSFC 0,661
  y 1,9. La dinámica de potencia es la de Stevens y Lewis. Se añade la
  recuperación de presión de una toma de choque normal fija, que es lo que
  limita al F-16 cerca de Mach 2.
- FLCS: mismas rutas de mando que `Systems/jsb-controls.xml` (gradiente de
  fuerza en palanca a G, límites +9/-3 G, limitador de AOA a partir de 22,5°
  con tope cerca de 25°, mando de régimen de alabeo de hasta 308 °/s reducido
  con AOA, interconexión alerón-timón, amortiguador de guiñada) con actuadores
  limitados en recorrido y velocidad. Los lazos internos de cabeceo y alabeo
  invierten las propias tablas aerodinámicas en lugar de reproducir la red de
  ganancias analógica, para que la respuesta sea estable de 120 nudos a Mach 2.

`cargo test --release level_speed -- --nocapture` resuelve el equilibrio de
vuelo nivelado con plena postcombustión y media carga de combustible:

| Altitud | Mach máximo nivelado |
| --- | --- |
| Nivel del mar | 1,24 |
| 10.000 pies | 1,47 |
| 20.000 pies | 1,68 |
| 30.000 pies | 1,90 |
| 40.000 pies | 2,02 |

Limitaciones: no hay programa de flaps de borde de ataque ni tablas de
pérdida más allá de 45° de AOA (se añade un momento de picado de placa plana),
el centro de gravedad del modelo visual está algo adelantado respecto al real,
y no se modelan límites térmicos, cargas externas, daños ni viento.

### Proporciones del modelo exterior

Las estaciones se midieron en el dibujo de tres vistas de Wikimedia Commons
(*General Dynamics F-16 Fighting Falcon 3-view line drawing*), escalado a
15,06 m: parabrisas al 17,8 % de la longitud del fuselaje, final de la cúpula
al 41 %, labio de la toma al 29 %, borde de salida del ala al 75 %, y una
profundidad de cúpula a fondo de toma de 0,15 veces la longitud. El eje del
motor, los estabilizadores y la deriva quedan por encima de la línea del morro.
La envergadura y la altura usan las cifras publicadas (9,96 m con AIM-9 y
4,88 m), porque el dibujo no está a escala exacta en esas cotas.

## Verificación

- `cargo test --release`, en `src/sim.rs` y `src/aero.rs`: signos de las
  superficies, interpolación de tablas, empuje y dinámica del motor, trimado
  a 1 G sin manos, 9 G en un segundo sin sobrepasar el límite, limitador de
  AOA a 25°, G negativo, alabeo coordinado de unos 300 °/s, combustible
  agotado, postcombustión, aerofreno, estabilidad numérica en la envolvente,
  velocidad máxima nivelada, cruce de puertas y puntuación.
- `src/terrain.rs`: la rejilla de 32 m coincide con la función analítica en los
  vértices y se desvía menos de 6 m entre ellos; la colisión usa los mismos
  triángulos que se dibujan.
- `src/airframe.rs`: dimensiones y proporciones medidas, geometría finita,
  normales unitarias, coordenadas de textura válidas y ojo de diseño dentro de
  la cúpula.
- Capturas con `--shot`, `--screen` y `--look` del menú, pausa, resultados,
  cabina y vista externa, y `--bench` para medir el tiempo por fotograma.
- Inspección visual de frente, ambos laterales, piloto y cola. Alt más mouse
  permite girar la cabeza hasta 155 grados por lado y 85 grados en vertical.
  Al soltar Alt, la vista vuelve al frente y la cámara externa recupera su
  posición normal. C también centra la vista. La cabina y el exterior comparten posición,
  escala y actitud; la cúpula opaca del modelo externo se omite en primera persona.
