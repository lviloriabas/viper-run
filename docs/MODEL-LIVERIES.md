# Texturas de las cuatro aeronaves, 29 de octubre de 2026

## Super Hornet y Growler

El F/A-18E lleva la librea VFA-22 Fighting Redcocks y el EA-18G la VAQ-129
Vikings. Son las imágenes originales del paquete FlightGear F-18F/G de
Petar Jedvaj y F-GTUX, con el permiso GPL incluido en el repositorio.
[Procedencia, autores y revisión del paquete](HORNET-CANOPY-SUPER.md).
La pintura VFA-22 procede del F del paquete y se adapta al E del juego; no
pretende reproducir una matrícula individual de esa variante.

La corrección importante es la orientación: AC3D define V desde abajo y la
imagen QOI se carga desde su primera fila superior. El importador anterior
conservaba V sin convertirla. Por eso aparecían franjas, insignias y bloques
de color en piezas equivocadas, aunque los dos archivos de imagen eran distintos.
Ahora se usa `1 - V`, con centros de texel y un atlas propio por variante.

El E conserva su imagen de 2048 x 2048. El Growler usa un atlas de 2048 x 2560:
la librea de 2048 x 2048 y, debajo, la imagen original de 512 x 512 de los ALQ-218.
Se conservan sus detalles y bandas. Cada rectángulo tiene sus propias UV; las
coordenadas no cruzan de la pintura del fuselaje a la de los pods. Las toberas,
los conductos oscuros, los neumáticos y el marco mantienen materiales separados.

## F-16 y MiG-29

El F-16C Block 50 usa la pintura 91-0352, 52FW/480FS de Spangdahlem, del
[F-16 de FlightGear mantenido por Nikolai V. Chr. y sus colaboradores](https://github.com/NikolaiVChr/f16).
La revisión fijada es `ef4f192f06243cf4061c2be794511db0576ac2f9`.
Se adaptan los paneles, registros, rótulos, insignias y códigos de cola.
El [copyright del paquete](https://github.com/NikolaiVChr/f16/blob/ef4f192f06243cf4061c2be794511db0576ac2f9/copyright.txt)
incluye sus archivos bajo GPL versión 2 o posterior; los autores completos
se conservan en `source/liveries/f16/authors.txt`.

El MiG-29 recibe la pintura gris y verde del
[MiG-29 de Sergey Salov y Enrique Laso Leon](https://github.com/Mercenary-Mercury/MiG-29_9-12),
revisión `d4a299bf88412f579b2067874989a8088b2f6d66`.
Su [declaración original](https://github.com/Mercenary-Mercury/MiG-29_9-12/blob/d4a299bf88412f579b2067874989a8088b2f6d66/ReadMeEng.txt)
licencia el exterior bajo GPL versión 3. Sustituye el camuflaje digital generado
por shader. Se conservan las estrellas, radomo, paneles dieléctricos, cristales,
pozos oscuros y metal de los motores del modelo del juego.

La fuente de pintura del MiG corresponde al 9.12; se adapta a la geometría 9.13S
del juego, sin cambiar su silueta ni afirmar una matrícula de servicio concreta.
Los F-16 agresores conservan sus cuatro pinturas especiales del juego.

## Adaptación y materiales

Se proyectan las mallas donantes originales con sus UV hacia cuatro vistas
del atlas: superior, inferior, derecha e izquierda. Un búfer de profundidad
escoge la piel visible y la interpolación baricéntrica busca su color en la
imagen original. No se utiliza una captura de pantalla como textura.
La selección de vista en el avión usa las normales que realmente se dibujan.
Algunas piezas simétricas comparten orden de vértices aunque sus caras miren
en sentidos opuestos; usar ese orden invertía rótulos y confundía la pintura
superior con la inferior. Las pruebas comprueban ambos lados de la deriva.

El atlas de cada avión es de 2048 x 1024, con cuatro áreas de 1024 x 512.
La textura se asigna a la piel del modelo procedural que ya existía. Las piezas
móviles conservan las UV de su posición de reposo y se desplazan con ellas.
Los ajustes de coordenadas, límites de las cuatro proyecciones, cobertura,
hashes y revisiones están en `assets/models/livery-sources.json`.
Son ajustes para transferir pintura; no sustituyen la geometría del avión.

El lienzo blanco conectado al fondo del F-16 no se considera pintura. Se
conservan los detalles blancos aislados de las insignias. Las islas de textura
del tren, cristales y calcomanías independientes del donante se excluyen.
En el MiG también se excluye la isla medida de las ruedas, columnas 0 a 799
y filas 1970 a 2047 de `MiG-29_2.png`, que algunos tapones del donante usan
sin un mapeo de pintura propio. Su verde brillante no aparece en el avión.

Cada vista extiende sus colores de borde hasta 32 texeles para cubrir pequeñas
diferencias de contorno, sin pasar a los otros rectángulos. Se utiliza una
paleta de 96 colores sin ruido ni tramado, conservando las líneas finas de la
fuente. Las cuatro texturas especiales de agresores permanecen en 512 x 256
para evitar aumentar su memoria sin ganar detalle.

## Reproducción y comprobación

Los originales usados, créditos y licencias están en
`assets/models/source/liveries/`. Se conservan sus bytes y hashes; las mallas
AC3D se guardan comprimidas sin pérdida. El juego funciona sin descargas,
Python ni Blender.

```sh
python -m pip install -r tools/requirements-models.txt
python tools/import_flightgear.py --unpack-source target/restored-models
python tools/import_liveries.py --source-root target/restored-models/liveries
python tools/import_superhornet.py --source-root target/restored-models/superhornet
python -m unittest discover -s tools -p 'test_*.py'
cargo test --all-targets
./build.sh
python tools/check_livery_views.py dist/viper-run-linux-x86_64 --output target/livery-review/linux
```

En Windows, `check_livery_views.py --geometry PATH` también exporta las mallas
exactas y copias PNG de sus atlas para Blender. `tools/review_liveries.py`
crea cuatro escenas editables con geometría pintada y referencia original,
además de vistas de ambos lados, superior, inferior y oblicua.
La exportación agrupa las escrituras para que las rutas UNC de Windows no
necesiten una operación de disco o red por cada número de cada vértice.
Los resultados finales y tamaños se registran en [VALIDATION.md](VALIDATION.md).
