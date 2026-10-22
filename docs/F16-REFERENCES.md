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
  unos 15 grados, desde un bulbo grueso junto a la palanca, por una cintura,
  hasta la cabeza de mandos en el extremo interior. Arriba de la cabeza van la
  perilla MAN RNG/UNCAGE, DOG FIGHT y el interruptor de comunicaciones; en el
  extremo, la rueda ANT ELEV; en la cara trasera, el cursor del radar y el
  aerofreno. La palanca de corte queda por fuera. La mano izquierda descansa
  plana encima, alineada con el antebrazo: una sola pieza de guante que cae
  por delante, y el pulgar por la cara trasera hacia el cursor.
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
| FCR | Contactos dentro de ±60° y de la escala (10 a 80 NM), con aspecto; el blanco fijado con su velocidad y altitud |
| RWR | Radares de los bandidos (16), AAA (A) y radar del SA-6 (6), rombo si apuntan o disparan, M por misil en vuelo |
| Interruptor MASTER ARM | SAFE en NAV, ARM con un arma seleccionada |

El HUD muestra el modo NAV y, con un arma seleccionada, los modos maestros
GUN, SRM, MRM y CCIP descritos en la sección de armamento. FCR muestra los
contactos aéreos (o los objetivos de tierra en CCIP) y RWR las amenazas de la
misión; en la carrera de puertas no hay contactos ni emisores. No se simulan
ILS, navegación INS con errores, TACAN ni comunicaciones. Los pulsadores del ICP y conmutadores de consola son
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
y no se modelan límites térmicos ni viento. Las cargas externas cambian la masa
y la resistencia, pero no las inercias ni la estabilidad; los daños son puntos
de estructura, sin averías de sistemas.

### Proporciones del modelo exterior

Las estaciones se midieron en el dibujo de tres vistas de Wikimedia Commons
(*General Dynamics F-16 Fighting Falcon 3-view line drawing*), escalado a
15,06 m: parabrisas al 17,8 % de la longitud del fuselaje, final de la cúpula
al 41 %, labio de la toma al 29 %, borde de salida del ala al 75 %, y una
profundidad de cúpula a fondo de toma de 0,15 veces la longitud. El eje del
motor, los estabilizadores y la deriva quedan por encima de la línea del morro.
La envergadura y la altura usan las cifras publicadas (9,96 m con AIM-9 y
4,88 m), porque el dibujo no está a escala exacta en esas cotas.

## Armamento, sensores y misiones de combate

Simplificado, pero con las cifras publicadas. La pantalla de inicio elige la
misión (GATE RUN, DOGFIGHT o STRIKE); las dos de combate pasan antes por una
pantalla de carga: las parejas de soportes exteriores (estaciones 3 y 7) e
interiores (4 y 6), el combustible interno (50, 75 o 100 %) y el peso bruto,
la resistencia añadida y la relación empuje/peso que resultan. La carrera de
puertas lleva los dos AIM-9 de punta de ala de siempre, de modo que su vuelo
no cambia.

| Arma | Datos usados | Fuente |
| --- | --- | --- |
| Cañón M61A1 | 511 disparos de 20 mm, 6.000 disparos por minuto, unos 1.036 m/s en boca, proyectil PGU-28/B de unos 100 g con Cd 0,28 (quedan unos 600 m/s a 1.000 m); boca en la raíz izquierda del ala, en (-0,72; 0,52; -3,40) m; línea de tiro por la cruz del cañón del HUD; los casquillos vuelven al tambor | [Ficha de la USAF del F-16](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104505/f-16-fighting-falcon/), [M61 Vulcan](https://en.wikipedia.org/wiki/M61_Vulcan), [PGU-28/B](https://www.globalsecurity.org/military/systems/munitions/pgu-28.htm) |
| AIM-9M Sidewinder | 85,5 kg, 2,87 m, 127 mm; motor Mk 36 Mod 11 de humo reducido, unos 5 s y 10,1 kN; Mach 2,5 a 2,7; 35 G; buscador infrarrojo de aspecto total con cardán de 40°; ojiva WDU-17/B de 9,4 kg con espoleta de proximidad DSU-15 | [Ficha de la Marina de EE. UU.](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2169316/aim-9-sidewinder/), [AIM-9](https://en.wikipedia.org/wiki/AIM-9_Sidewinder) |
| AIM-120C AMRAAM | 161,5 kg, 3,66 m, 178 mm; motor de aceleración y sostenimiento (24 kN 2,6 s, 9,8 kN 6 s) hasta Mach 3 a 4; 40 G; guiado inercial con el blanco del radar y buscador radar activo que busca solo si se lanza sin blanco; ojiva WDU-41/B de 18 kg | [Ficha de la USAF del AIM-120](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104576/aim-120-amraam/), [AIM-120](https://en.wikipedia.org/wiki/AIM-120_AMRAAM) |
| Mk-82 | 241 kg con espoleta, 2,21 m, 273 mm, 87 kg de Tritonal; Cd 0,22; eyección de 1,5 m/s; espoleta armada a los 2 s; letal hasta 8 m, daño hasta 45 m | [Mk 82](https://en.wikipedia.org/wiki/Mark_82_bomb), [GlobalSecurity](https://www.globalsecurity.org/military/systems/munitions/mk82.htm) |
| Contramedidas | 30 cartuchos de chaff y 30 bengalas (simplificación de los lanzadores ALE-47); cada pulsación suelta uno de cada | [AN/ALE-47](https://en.wikipedia.org/wiki/AN/ALE-47) |

Los misiles vuelan con navegación proporcional verdadera (N = 4): la
aceleración lateral es N por la velocidad de cierre por la velocidad angular
de la línea de visión, más la sustentación que compensa la gravedad. La limitan
el límite estructural de G y la fuerza normal que el fuselaje da con su ángulo
de ataque máximo a la presión dinámica del momento, de modo que un misil lento
o alto gira poco. Esa fuerza normal produce resistencia inducida (N por el
ángulo de ataque), además de la resistencia de onda que sube en el transónico,
así que un misil que maniobra mucho pierde energía. La espoleta se arma a los
0,5 s (AIM-9) o 1 s (AIM-120) y estalla en el punto de máxima aproximación si
pasa a menos de 9 o 12 m; el daño cae linealmente con la distancia. Las
pruebas dan alcances con cola de unos 8 km para el AIM-9M a 1.000 m y unos
11 km para el AIM-120C, más con altitud y de frente.

- El visor del cañón sigue la idea del EEGS: proyectiles virtuales disparados
  cada 1/30 s con la balística real forman el embudo, escalado para 35 pies de
  envergadura, y el retículo LCOS está sobre esa corriente a la distancia del
  blanco fijado (2.000 pies sin blanco). Es exacto si el tirador mantiene el
  seguimiento durante el tiempo de vuelo.
- El CCIP vuela una bomba completa con el mismo paso y la misma balística que
  una real, de modo que la bomba lanzada sobre el retículo cae en él (lo
  comprueba una prueba). La línea de caída va del marcador de trayectoria al
  retículo.
- El AIM-9 busca en un campo de 2,5° a lo largo del eje del avión o donde mira
  el casco, y se esclaviza al blanco del radar dentro de su cardán de 40°. El
  aviso IN RNG sale de simular el disparo contra la trayectoria actual del
  blanco; el piloto artificial usa la misma comprobación.
- Casco al estilo JHMCS: al girar la cabeza fuera del HUD aparecen una cruz de
  puntería, datos de vuelo, el recuadro del blanco y el buscador, que el casco
  orienta fuera del eje. T fija el blanco más cercano a la línea de visión.
- Las bengalas solo engañan a buscadores infrarrojos (probabilidad 0,35, por
  la lógica de rechazo del AIM-9M) y el chaff solo a buscadores radar, mejor
  con el blanco de través (entre 0,08 y 0,63); ninguna funciona con el misil
  a más de 6 km.

Los bandidos vuelan el mismo modelo de seis grados de libertad y el mismo FLCS
que el jugador; el piloto artificial solo decide adónde va el vector de
sustentación, cuánto tirar y dónde va el acelerador. Usa maniobras básicas de
combate: persecución adelantada para el cañón, pura para cerrar y retrasada
para no pasarse, viraje máximo cuando el jugador está a sus seis, haz y luego
viraje hacia un misil, y recuperación de energía. Sobre todo eso manda una
predicción del tipo Auto-GCAS del F-16: vuela por adelantado el tirón de
recuperación (alabeo y 6 G) y lo ejecuta cuando quedaría a menos de 180 m del
terreno, eligiendo entre subir recto o virar si una pared del valle lo exige.
Dispara el cañón dentro de 900 m con solución de tiro, lanza AIM-9 dentro de
la zona de lanzamiento y suelta contramedidas cuando un misil se acerca.

Objetivos de tierra en STRIKE, junto al río: convoy de cuatro camiones con un
ZSU-23-4, batería SA-6 (lanzador 2P25 y radar 1S91) con camión y otro ZSU, el
puente de la carretera con un tercero, y un depósito con almacenes y tanques.
El ZSU-23-4 usa cuatro cañones de 23 mm, 3.400 disparos por minuto en total,
970 m/s y unos 2,5 km de alcance eficaz; dispara ráfagas con trazadoras y
sigue con retraso. El misil 3M9 del SA-6 pesa 599 kg, mide 5,8 m y 335 mm,
alcanza Mach 2,8 con su estatorreactor integrado, lleva 57 kg de carga y es
semiactivo: deja de guiar si el radar muere. La batería solo dispara si su
radar ve el avión a más de 300 m sobre el terreno, así que el valle protege.
Fuentes: [2K12 Kub](https://en.wikipedia.org/wiki/2K12_Kub) y
[ZSU-23-4](https://en.wikipedia.org/wiki/ZSU-23-4_Shilka).

Limitaciones: no hay radar con modos reales (RWS, TWS, ACM), enlace de datos,
IFF, láser, bombas guiadas ni HARM; las contramedidas no se ven en el radar
del enemigo; la masa de las cargas no cambia las inercias; el daño es un
contador de puntos de estructura sin averías de sistemas; los bandidos son
F-16 con pintura de agresor y su RWR se representa como radar "16".

## Verificación

- `cargo test --release`, en `src/sim.rs` y `src/aero.rs`: signos de las
  superficies, interpolación de tablas, empuje y dinámica del motor, trimado
  a 1 G sin manos, 9 G en un segundo sin sobrepasar el límite, limitador de
  AOA a 25°, G negativo, alabeo coordinado de unos 300 °/s, combustible
  agotado, postcombustión, aerofreno, estabilidad numérica en la envolvente,
  velocidad máxima nivelada, cruce de puertas y puntuación.
- `src/weapons.rs`: tiempo de vuelo, velocidad y caída del proyectil de 20 mm;
  impactos por barrido contra el elipsoide del avión y cajas; daño por
  distancia; ambos misiles alcanzan blancos sin maniobra por detrás y de
  costado; alcances finitos y mínimos; el CCIP coincide con la bomba simulada;
  los proyectiles disparados al retículo LCOS impactan; cada contramedida
  engaña solo a su tipo de buscador.
- `src/ai.rs`: la ley de mando gira a un rumbo; el piloto artificial no choca
  con el terreno en el valle durante 90 s; dos pilotos combaten dos minutos
  sin chocar ni pasar de los límites; un as sigue un blanco con el cañón.
- `src/combat.rs` y `src/targets.rs`: masa y resistencia de las cargas, ciclo
  de modos, fijación del blanco más cercano al eje, daño y derribos, un AIM-9
  fijado derriba a un bandido, una Mk-82 lanzada sobre el retículo derriba el
  puente, objetivos sobre el fondo del valle y blindaje frente al cañón.
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
