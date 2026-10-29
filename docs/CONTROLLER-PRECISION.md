# Precisión del mando y calibración

Abre Controls / Config y CONTROLLER SETUP. Está disponible en el menú y en vuelo, también con ratón. Durante la configuración se pausa el vuelo local; en multijugador la sala continúa y los controles locales quedan suspendidos.

- LEFT / RIGHT DEAD ZONE: zonas muertas radiales independientes, del 0 al 35 %, en pasos del 1 %. El monitor muestra la lectura original y la salida después del filtrado y la curva.
- AUTO CALIBRATE DRIFT: opción manual dentro del juego. Deja ambos sticks sin tocar durante 4,5 segundos. Hay 1,5 segundos iniciales para soltarlos y tres de medición. Se toma el máximo radial de cada stick, se suma un margen de 1,5 puntos porcentuales y se redondea hacia arriba al porcentaje entero. Se aplica y guarda solo una medición completa válida.
- Si se detecta movimiento, una desconexión, pocas muestras o una interrupción, se conservan los valores anteriores. B, Esc y CANCEL CALIBRATION cancelan. El calibrador mide drift en reposo; no mide el recorrido completo. Al cambiar de mando conviene repetirlo.
- CENTER SOFTNESS: curva progresiva con respuesta reducida cerca del centro, sin retrasar la entrada y conservando todo el recorrido. El valor inicial nuevo es 65 %; se respetan los valores ya guardados.
- FINE AIM WITH ZOOM: aumenta progresivamente la suavidad central mientras se mantiene el zoom. Puede desactivarse. El stick derecho también tiene una curva suave para ajustar mejor la mirada.
- X OR L3 FOR RUDDER: en COMBAT conserva X más stick izquierdo y añade la alternativa de mantener pulsado el clic L3 mientras se mueve a izquierda o derecha. Ambas funcionan cuando se activa esta opción. Para evitar contramedidas involuntarias, se lanzan con X más L3. Los dos clics juntos conservan el interruptor de limitadores. CLASSIC mantiene sus asignaciones.
- RESET CONTROLLER restaura solo estos ajustes. Los valores se guardan en el archivo habitual de preferencias del usuario. La zona muerta inicial sigue siendo 10 % para evitar drift en mandos desconocidos.

## Investigación y decisiones

Microsoft explica que las lecturas centradas varían entre mandos y recomienda filtrar y reescalar su magnitud. Se mantiene una zona radial continua para conservar la dirección, sin saltos al salir del centro. La calibración utiliza los datos originales, antes de cualquier filtro, para poder detectar drift menor que la antigua zona fija.

Fuente: [Microsoft, Getting Started With XInput, Dead Zone](https://learn.microsoft.com/en-us/windows/win32/xinput/getting-started-with-xinput#dead-zone).

Steam Input documenta controles separados para zona muerta y curva de respuesta. Aquí también se separan: la zona muerta elimina lecturas involuntarias y la curva distribuye el recorrido útil para permitir correcciones finas. No se añade una zona muerta por eje ni aceleración temporal al control de vuelo. La precisión con zoom conserva la autoridad máxima; su efecto se concentra cerca del centro.

Fuente: [Valve, Input Source Modes](https://partner.steamgames.com/doc/features/steam_controller/input_source_modes).

La curva inicial de 65 %, el margen de calibración y la mayor suavidad con zoom son decisiones de este juego, no valores recomendados por esas fuentes. La comodidad real debe comprobarse con el mando del jugador y puede ajustarse en el monitor.

## Verificación

Las pruebas cubren continuidad y dirección de la zona radial, recorrido máximo, detección independiente del drift, rechazos y cancelación, persistencia de ajustes, precisión con zoom y separación entre timón y contramedidas. La prueba de integración de menú abre el configurador con ratón, modifica valores con mando y teclado, verifica la entrada de timón L3 y cancela una calibración. Los escenarios de distribución visual incluyen configuración, medición activa y timón L3.
