# Cabina del MiG-29

Revisión del 22 de octubre de 2026. La cabina del MiG-29S (9.13S) es propia,
no una copia de la del F-16. Las fotografías se usan como referencia visual y
no se incluyen como texturas ni se redistribuyen.

## Referencias visuales

Fotografías de cabinas reales del 9.12 en Wikimedia Commons (el 9.13 conserva
casi sin cambios la misma cabina):

- [MiG-29 cockpit 3](https://commons.wikimedia.org/wiki/File:MiG-29_cockpit_3.jpg):
  la cabina completa desde arriba. Fija la disposición general: panel y tubo
  en gris azulado, palanca central entre las piernas, consolas laterales a la
  altura de los muslos, el HUD ILS-31 con su marco grueso sobre la visera
  negra, la pantalla IT-23 a su derecha y la columna de luces de aviso junto
  al riel derecho de la cúpula.
- [Polski MiG-29 kokpit](https://commons.wikimedia.org/wiki/File:Polski_mig_29_kokpit.JPG):
  panel de un MiG-29 polaco modernizado; instrumentos redondos de fondo negro,
  el KPP (director de actitud) sobre el PNP (situación horizontal) en el
  centro, anemómetro y altímetro a la izquierda.
- [MiG-29 cockpit 1](https://commons.wikimedia.org/wiki/File:MiG-29_cockpit_1.jpg):
  la empuñadura de la palanca: negra y cuadriculada, con la cabeza
  ensanchada hacia delante, el botón rojo de armas, el compensador y la
  palanca del freno de ruedas por delante.
- [MiG-29 cockpit 2](https://commons.wikimedia.org/wiki/File:MiG-29_cockpit_2.jpg):
  detalle del panel derecho: temperatura de gases doble en x100 °C,
  indicadores de hidráulica y oxígeno y la posición de la rampa de admisión.
- [MiG-29 cocpit 1](https://commons.wikimedia.org/wiki/File:MiG-29_cocpit_1.jpg)
  y [cockpit 114th brta](https://commons.wikimedia.org/wiki/File:-29_cockpit_114th_brta.jpg):
  el arco grueso de la cúpula con sus tres espejos retrovisores y la bola
  IRST delante del parabrisas, a la derecha.

## Disposición en el juego

La cabina se construye en el mismo espacio que la del F-16, con el ojo de
diseño en el mismo punto, y `cockpit::body_camera` lleva la cámara desde el
ojo del MiG (`mig29::EYE`) hasta allí; así el asiento, el cuerpo del piloto,
las piernas y el HUD se comparten. El exterior recorta la piel en
`mig29::CUTOUT` desde el asiento y no dibuja la cúpula, de modo que nada
tapa la vista delantera salvo el morro y la bola IRST.

- Panel inclinado 12 grados, con alas plegadas 22 grados hacia el piloto.
  Centro: KPP sobre PNP; a la izquierda anemómetro en km/h con ventana de
  Mach, altímetro en metros (aguja larga por kilómetro, corta por diez y
  contador) y el UUAP (ángulo de ataque arriba, factor de carga abajo); a la
  derecha variómetro en m/s, tacómetro doble y temperatura de gases doble.
  Ala derecha: combustible en kg, hidráulica y reloj. Ala izquierda: luces del
  tren y flaps y el panel de piloto automático y navegación (SAU, RSBN, ARK).
- Visera negra con el HUD ILS-31 (cuerpo grande y marco grueso alrededor del
  cristal combinador) y la pantalla IT-23 con una imagen tipo B del radar.
- Columna de diez luces de aviso junto al riel derecho; las leyendas van
  transliteradas porque la fuente del juego solo tiene caracteres latinos.
- Palanca central sobre el suelo, con fuelle, y acelerador de dos palancas en
  un cuadrante sobre la consola izquierda; palanca del tren en la pared
  izquierda, delante del acelerador.
- Marcos de la cúpula sobre la forma exterior (`mig29::canopy_point`): postes
  del parabrisas, el arco sobre las rodillas con sus tres espejos y los
  rieles; mamparo detrás del asiento y la cubierta bajo la parte trasera de
  la cúpula.

## Armamento del MiG-29A 9.12

Cifras publicadas que usa el juego (`weapons.rs`, `combat.rs`):

- Cañón GSh-30-1 (9A-4071K) en la raíz del LERX izquierdo: 150 proyectiles
  de 30 x 165 mm, 1.500 a 1.800 disparos por minuto, el OFZ de 390 g a
  860 m/s.
- R-73 (izdeliye 72, AA-11 Archer): 105 kg, 2,90 m, 170 mm; cabeza infrarroja
  refrigerada todo aspecto a 45 grados del eje, guiada por el visor de casco
  Shchel-3UM; ojiva de varillas de 7,4 kg y espoleta radioeléctrica.
- R-27R (izdeliye 470R, AA-10 Alamo-A): 253 kg, 4,08 m, 230 mm; guía inercial
  y luego semiactiva sobre el radar N019, que debe mantener el blanco hasta el
  impacto; ojiva de varillas de 39 kg; hasta 73 km de frente en altura.
- FAB-250M-62: 250 kg, 1,92 m, 300 mm, 100 kg de TNT. FAB-500M-62: 497 kg,
  2,47 m, 400 mm, unos 200 kg de TNT.
- Dos dispensadores BVP-30-26M de 30 cartuchos de 26 mm (PPI-26 bengalas,
  PPR-26 chaff).
- Seis pilones subalares: R-73 en los exteriores, R-73 o FAB-250 en los
  medios, R-27R o bombas pesadas en los interiores; carga típica aire-aire de
  cuatro R-73 y dos R-27R.

Fuentes: [R-27 (Wikipedia)](https://en.wikipedia.org/wiki/R-27_(air-to-air_missile)),
[FAB-500](https://en.wikipedia.org/wiki/FAB-500),
[FAB-250](https://en.wikipedia.org/wiki/FAB-250),
[FAB-500M-62 (GlobalSecurity)](https://www.globalsecurity.org/military/world/russia/fab-500m-62.htm).
