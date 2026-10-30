# Aros, montajes y variantes Hornet, 29 de octubre de 2026

## Aro, espejos y brújula

El aro del F/A-18C conserva sus contactos originales con los laterales, pero
su sección completa es ahora una curva oval continua, de 0.380 m de semiancho
y 0.553 m de altura. Se eliminan los postes rectos que quedaban debajo del
antiguo semicírculo. Son dimensiones ajustadas a las referencias visuales y a
la envolvente del modelo, no cotas de fabricación.

Los espejos laterales pasan de 180 x 60 mm a 212 x 41 mm y el superior de
205 x 52 mm a 222 x 38 mm. Sus cajas y vidrios siguen el aro, con dos bisagras
cortas y cálculo óptico a partir de las normales y del ojo del piloto.
Se verifica el vidrio completo desde el ojo de diseño y con desplazamientos
de cabeza de 15 mm horizontales y 10 mm verticales.

La brújula tiene su placa de fijación y su soporte en el propio arco derecho.
La posición de la raíz se obtiene del mismo contorno que genera el aro. No
depende del antiguo punto suelto del modelo importado. Las variantes E/G usan
también su carcasa horizontal y escala correspondiente.

Referencias: [NATOPS del Hornet, sección 2.12.9](https://server.3rd-wing.net/public/Bureau_4thMEG/VMFA-232/Documentation/Support/A1-F18AC-NFM-000_-_NATOPS_Flight_Manual_-_February_2000.pdf),
[NATOPS E/F, sección 2.18.5, página I-2-141](https://info.publicintelligence.net/F18-EF-000.pdf)
y [fotografía real de la Navy 050705-D-D0439-7026](https://www.dvidshub.net/image/9214059/plane-captain-cleans-canopy-f-18c-hornet).
Los manuales sitúan la brújula en el arco derecho del parabrisas. La fotografía
sirve para contrastar curvatura, proporción de los espejos y asientos sobre el
borde interior; sus medidas se ajustan en la escena 3D.

## Exteriores propios del Super Hornet y Growler

El F/A-18E y el EA-18G ya no seleccionan el exterior del Hornet C. La base es
el [paquete FlightGear F-18F/G publicado por F-GTUX](https://forum.flightgear.org/viewtopic.php?f=4&t=42475),
actualizado el 13 de marzo de 2026, con geometría de Petar Jedvaj y autorización
expresa del autor para su uso bajo GPL. El
[ZIP público](https://drive.google.com/file/d/1wqAzllmGU604xC8W5gxYNv2dGL5oSaw7/view)
tiene SHA-256 `57d3ac9064617f98aa6478051a3be98c41a50726dcdf15a8e0a2df60499740a2`.
La autorización y los originales utilizados se conservan en
`assets/models/source/superhornet/`; los hashes están en
`assets/models/superhornet-sources.json`.

Se contrastó el E con el [modelo mostrado por sus desarrolladores en War Thunder](https://warthunder.com/en/news/9853-development-fa-18e-super-hornet-the-hornet-queen-en)
y su [ficha de variante Block II](https://wiki.warthunder.com/unit/fa_18e_block_2).
War Thunder se utiliza como referencia visual; sus mallas y texturas no se
incorporan al juego. Las diferencias de escala y tripulación se contrastaron
con [Boeing](https://www.boeing.com/defense/fighters-and-bombers/fa-18-super-hornet-and-ea-18-growler)
y [NAVAIR, EA-18G](https://www.navair.navy.mil/product/EA-18G-Growler).

La adaptación conserva entradas rectangulares, LEX, alas y colas mayores,
seis superficies delanteras/alerones separadas, estabilizadores, flaps,
timones y tren propios. El E tiene una canopy de un asiento, acortada 1.31 m
respecto a la fuente F/G, con la zona posterior cerrada por el lomo. El Growler
conserva la canopy larga, los dos ocupantes, receptores ALQ-218 en las puntas,
antenas y cercas de ala propias. Las libreas proceden de VFA-22 y VAQ-129 del
mismo paquete, adaptadas a la paleta plana del juego.
La [revisión de texturas](MODEL-LIVERIES.md) corrige el origen vertical de las
UV, coloca las insignias y rótulos sobre sus piezas y añade el atlas original
de los receptores ALQ-218 del Growler.

El aerofreno dorsal que el artista incluyó en la fuente no existe como
superficie móvil en estas variantes. Se elimina su caja y se mantiene una
cubierta fija; el freno actúa mediante las superficies de vuelo, incluido
el movimiento simétrico de los timones.

## Coordenadas, montajes y materiales

Ejes de juego: derecha, arriba, atrás, en metros. Para un punto AC3D `(x,y,z)`:

```
derecha = -z * (13.7 / 13.1799)
arriba = (y - 2.20) * (4.88 / 5.14935)
atrás = x * (18.3 / 18.26665) + 0.15
```

La escala usa la extensión del modelo frente a las dimensiones nominales.
Las posiciones del ojo se ajustan al piloto y panel reconstruidos dentro de
esa envolvente: coordenadas fuente `(-3.70, 2.85, 0)` delante y
`(-2.32, 3.00, 0)` detrás. Son ajustes de integración; el paquete advierte que
su cabina original no es representativa. La misma transformación se usa para
la cámara, cuerpo del piloto, asientos, canopy, ruedas, contactos, toberas y
efectos. No se desplaza sólo la cámara para ocultar una intersección.

Las piezas importadas de armas y depósitos se excluyen del exterior fijo.
Cada arma la dibuja una sola estación del inventario. Los centros de montaje
se obtienen de los pilones separados, corrigiendo el origen en la nariz de
los misiles. Pilones y armas de E/G comparten 3.5 grados de apertura lateral;
los rieles de las puntas siguen su incidencia. Los pequeños soportes de
suspensión ocupan los 45 mm entre el riel y el cuerpo del misil. El montaje
de la punta del E es lateral, con 40 mm para sus herrajes. Los ALQ-218 del
Growler reservan las puntas; sus misiles se montan bajo las alas.

Las tres parejas seleccionables de E/G corresponden a los tres pilones bajo
cada ala. Las estaciones adicionales de fuselaje/centro no son nuevas filas
de carga en esta revisión. La ampliación Fox 1/2/3 y sus diferencias respecto
a las configuraciones reales se documentan en [WEAPONS.md](WEAPONS.md).

En el Hornet C se retrasa el riel de fuselaje de z=0.35 a z=1.10 m. Dos
soportes, en z=0.70 y z=1.45 m, contactan con la piel de las tomas en lugar
de quedar colgados bajo el espacio vacío del LEX. Sus armas usan el mismo
nuevo centro. Se preservan los anclajes bajo las alas.

El riel queda a y=-0.75 m y el centro del misil a y=-0.90 m. La separación
adicional evita que las aletas del Sparrow y AMRAAM penetren en el conducto.
Una prueba proyecta los vértices de las cinco familias de misiles occidentales
contra la piel original de ambas tomas, con 8 mm de tolerancia de contacto.

Las toberas F414 usan colores metálicos lisos y cavidades oscuras sin UV.
Los interiores importados del Hornet C y Su-27 también eliminan el muestreo
del atlas exterior. Los interiores procedurales existentes de F-16 y MiG-29
ya eran materiales lisos. Se separan los marcos opacos del vidrio, se
suprime la geometría sustituida y el exterior deja espacio al modelo de
cabina desde primera persona.

## Daño según su causa

La fatiga y las sobrecargas estructurales abren grietas largas con metal
expuesto en las zonas de los largueros. El daño por armas conserva impactos,
quemaduras y hollín localizados. Son canales independientes del shader:
`structure.damage` no alimenta la máscara de impactos o fuego de `battle`.
Pueden coexistir cuando el avión sufre ambos tipos de daño.

`--stress-preview` y `--damage-preview` permiten comparar ambos estados en la
misma aeronave y vista. Las marcas son una representación regional del daño;
no son fracturas de chapa simuladas individualmente.

## Reproducción y límites

```
python tools/import_flightgear.py --unpack-source target/model-source-restored
python tools/import_superhornet.py --source-root target/model-source-restored/superhornet
cargo test --all-targets
python -m unittest discover -s tools -p 'test_*.py'
./build.sh
python tools/check_hornet_revision.py dist/viper-run-linux-x86_64 --output target/canopy-super-review/linux
dist/viper-run-linux-x86_64 --export-cockpit target/canopy-super-review/geometry
blender -b --python tools/review_hornet_revision.py -- target/canopy-super-review/geometry
```

Las escenas separan exterior fijo, superficies móviles, inventario montado,
cabina, piloto y referencia original, con vistas completas, de montajes y del
ojo del piloto. Los neumáticos conservan dimensiones y ejes medidos con una
geometría reducida. Cada exterior queda por debajo de 30,000 triángulos.

La cubierta E derivada del F/G y el panel delantero son reconstrucciones
ajustadas con las referencias, no piezas industriales del fabricante. La
cabina trasera no es operable y los pods no implementan guerra electrónica.
El juego conserva las simplificaciones de armamento descritas en WEAPONS.md.
Las cifras finales, hashes y resultados están en [VALIDATION.md](VALIDATION.md).
