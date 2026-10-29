# Revisión del TODO, 29 de octubre de 2026

## Controles y carga

Los sliders de FOV, ratón, respuesta, zonas muertas y sensibilidad del mando
aceptan clic y arrastre. Conservan la captura aunque el puntero salga de la barra,
limitan el valor a sus extremos y dejan de modificarlo al soltar o cerrar el panel.
Los botones siguen disponibles para teclado y mando.

CONTROLLER SETUP separa sensibilidad de vuelo (0.5 a 2.0), sensibilidad de vista
(0.5 a 2.0), expo y zona muerta. El vuelo conserva toda la autoridad en los
extremos y una respuesta simétrica. Los ajustes sobreviven al reinicio.

Todas las parejas de armas, incluidas las puntas, admiten EMPTY. La fila del
cañón selecciona tambor completo, tres cuartos, mitad, un cuarto o cero. El
cañón permanece instalado. AUTO REFILL respeta la capacidad elegida; una carga
sin armas ni munición sigue vacía. Las estaciones vacías no dibujan misiles.

El modelo de vuelo estaba calibrado con el tambor completo. La contabilidad
mantiene esa referencia, resta los cartuchos omitidos en la configuración y,
al disparar, resta proyectil y propelente, conservando las vainas del sistema
sin eslabones. Se utilizan 0.26 kg por cartucho de 20 mm y 0.84 kg por cartucho
de 30 mm, con masas expulsadas de 0.14 y 0.55 kg respectivamente. Son valores
representativos de las municiones del juego, no un catálogo de todas sus variantes.
La carga inicial se conserva también al aplicar una instantánea multijugador.

## Protección de vuelo

El Hornet admite un régimen de AoA mayor: transición nominal desde 32 grados,
techo de juego de 38 grados y separación base a 42 grados. Esto permite el
apuntado a baja velocidad conservando el presupuesto compartido de cabeceo,
alabeo y guiñada. Mach, altura, resbalamiento, ráfagas y rotación de cada
semiala siguen reduciendo el margen disponible. La presión dinámica y el peso
determinan la G que físicamente se puede alcanzar.

El F/A-18E y el EA-18G usan una referencia de masa propia de 19,496 kg para
su límite de G, en lugar de los 14,515 kg del C. Sus motores usan los empujes
representativos del F414. El máximo nominal sigue en 7.5 G. Las referencias de
masa y la adaptación de AoA son calibraciones de esta simulación; no reproducen
las leyes completas del fabricante. F-16, MiG-29 y Su-27 conservan sus límites
nominales y el mismo sistema de protección adaptable.

El manual primario [NATOPS A1-F18AC-NFM-000](https://www.vmfa251.org/pdffiles/F18-ABCD-000.pdf)
describe autoridad y capacidad de apuntado por encima de 35 grados, junto con
reducción de alabeo en ciertas condiciones de Mach/AoA. No establece un límite
duro universal de 30 grados. Las dimensiones y motores de las variantes E/G
se contrastaron con [Boeing](https://www.boeing.com/defense/fighters-and-bombers/fa-18-super-hornet-and-ea-18-growler).

## Geometría, materiales y daños

Las tapas de cortes articulados se forman por contornos conectados y se
triangulan respetando concavidades. Ya no unen puntos de piezas separadas en
un único abanico. Los slats del Su-27 están restringidos al espesor del ala.
Se elimina el SpeedbrakePushrod importado: su animación independiente no se
podía tratar como parte rígida de la tapa del aerofreno.

Los morros y las zonas curvas seleccionadas reciben normales suavizadas con
conservación de aristas. Se elimina el grano del F-16, se suaviza el contraste
de sus líneas y se conserva la paleta plana de los materiales importados.
El camuflaje y las insignias del Su-27 permanecen en sus imágenes originales
adaptadas; no se incorporan imágenes de terceros nuevas.

Hornet y Su-27 tienen un pozo de cabina opaco visible desde fuera, unido a los
bordes reales de la canopy, con suelo y paredes de ambos lados. El renderer
E/G selecciona ahora la familia Hornet en lugar de caer en el modelo F-16.
Los mandos, HUD, instrumentos, espejos y páginas de esa familia se seleccionan
también para E/G. El panel frontal E/G usa una UFCD electrónica y no las teclas
físicas del C. La [Navy](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2166036/ea-18g-growler-airborne-electronic-attack-aircraft/ea-18g-growler-airborne-electronic-attack-aircraft/)
identifica al Growler como una variante derivada del F/A-18F de dos tripulantes.
Se muestran dos ocupantes y se secuencian dos asientos, empezando por el trasero.

Límite de alcance: E/G mantienen el exterior compacto compartido de la familia
Hornet y una reconstrucción simplificada de su panel delantero. No se ha
añadido un modelo industrial completo E/F/G, una cabina posterior operable ni
sistemas de guerra electrónica. La corrección de este TODO resuelve la falta
de materiales y la selección incorrecta de geometría e instrumentos.

El daño visual añade marcas persistentes en coordenadas del avión, con
oscurecimiento, impactos y quemaduras por región dañada (alas, motores y
estructura). Se combina con humo, fuego, flexión y desprendimientos existentes.
Las marcas representan el estado de cada región, no una perforación geométrica
individual por proyectil. `--damage-preview` permite capturarlas de forma repetible.

## Fallo estructural y escape

Perder un ala deja el vuelo activo y conserva la vista elegida. El piloto
puede tirar del mando de eyección mientras permanezca consciente. El daño
estructural no impone por sí solo un G-LOC: la fisiología sigue la aceleración
medida. Después del escape sigue la del asiento y del piloto, no la del avión
abandonado. El impacto real de la aeronave sigue terminando el vuelo si no
se ha producido la eyección.

Los asientos reciben amortiguación y un momento de estabilización. El drogue
se despliega en el régimen alto/rápido de ACES II y en NACES; el K-36 conserva
sus brazos y pequeños estabilizadores, con un drogue visible simplificado.
La secuencia de alta cota retiene el paracaídas principal hasta descender a:

| Asiento y aeronaves | Umbral utilizado |
| --- | --- |
| ACES II, F-16 | 4,570 m MSL, aproximadamente 15,000 ft |
| K-36DM, MiG-29 y Su-27 | 5,000 m MSL |
| NACES, F/A-18C/E y EA-18G | 4,570 m MSL, ajuste representativo del secuenciador |

La [USAF documenta un ACES II en modo III](https://media.defense.gov/2008/Jan/10/2001453265/-1/-1/0/F-15C_2Nov07_Vol_1_of_4%2520Updated.pdf)
con liberación del drogue cerca de 15,000 ft. El [Department of Commerce](https://www.bis.gov/media/documents/nationalsecurityassessmentemergencyaircraftejectionseatsectorfinal24.pdf)
describe los tres modos y el uso de estabilización y drogue. El
[Ministerio de Defensa de India](https://www.ddpmod.gov.in/hi/node/8038)
publica el techo de despliegue de 5,000 m AMSL del PSU-36 del K-36DM.
[NAVAIR confirma NACES para F/A-18 y EA-18G](https://www.navair.navy.mil/product/Navy-Aircrew-Common-Ejection-Seat-NACES).
Los tiempos, áreas de resistencia, actitud estabilizada, secuencia de dos
asientos y umbral NACES son aproximaciones del juego, no instrucciones de uso real.

El sonido continuo sigue al piloto: viento según su velocidad, atenuación con
la distancia al avión, variación Doppler y desaparición del tono del buscador.
El impacto del avión abandonado también atenúa su sonido con la distancia.

## Misiles que no interceptan

Antes se eliminaba un misil al bajar de Mach 1 y se dibujaba una explosión.
Ahora el agotamiento de energía no lo elimina: continúa con inercia,
resistencia y gravedad, pierde capacidad de maniobra y puede golpear el
terreno. Sin guiado ni motor ya no se le añade sustentación artificial para
mantener la trayectoria horizontal. Termina por impacto, proximidad armada
o el temporizador de vida del modelo. Las estelas se cierran cuando termina.
La predicción de IN RNG conserva un margen de energía terminal conservador.

[NAVAIR](https://www.navair.navy.mil/product/AMRAAM) describe las fases de guiado
del AMRAAM, pero no publica un temporizador de autodestrucción común para
todos los misiles del juego. Sus tiempos de vida son límites de simulación
por modelo; no se presentan como datos reales confirmados.

## Compilación y reproducción

Rust trata los warnings y mensajes del enlazador como errores. El controlador
`tools/zig-driver.sh` elimina únicamente `-Wl,-O1`, una opción de GNU ld que
el enlazador de Zig 0.15 declara obsoleta. No filtra su salida ni desactiva
diagnósticos. Windows requiere que se compile el recurso de icono; también
se establece identidad de aplicación e iconos de ventana y clase.

Las pruebas nuevas cubren carga vacía, masa inicial y consumida, instantáneas,
respuesta del mando, persistencia, fin de vida de misiles, estabilización,
retención por altitud, dos asientos, protección de las seis aeronaves y tapas
desconectadas. `--ui-check` usa los eventos nativos para arrastre, liberación,
carga limpia y eyección con daño estructural.

`tools/check_pending_visuals.py` genera la matriz de vistas nativas, con ajustes
aislados y hash del ejecutable. `--export-cockpit` exporta también los exteriores
ensamblados después de los cortes; `tools/review_pending_models.py` produce
escenas Blender con geometría del juego, cabina y referencia fuente separadas.
Los resultados finales y sus hashes se registran en VALIDATION.md.
