# Cabina del MiG-29

Revisión del 22 de octubre de 2026. La cabina del MiG-29A (9.12) es propia,
no una copia de la del F-16. Las fotografías se usan como referencia visual y
no se incluyen como texturas ni se redistribuyen.

## Referencias visuales

Fotografías de cabinas reales del 9.12 en Wikimedia Commons:

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
