# Multijugador, instrucciones de uso

## Inicio

Todos deben usar el juego actualizado. Abre ONLINE y elige CALLSIGN y avión.
El menú separa SERVERS, CREATE ROOM, JOIN INVITE y LAN DIRECT.

El navegador necesita un viper-server en ejecución y accesible. No hay un
servidor público predeterminado. Configuras su dirección una vez y se guarda.

## 1. Poner en marcha el servidor

En Windows abre viper-server-windows-x86_64.exe desde dist. En Linux:

~~~sh
./dist/viper-server-linux-x86_64 0.0.0.0:42861
~~~

Solo la máquina del servidor necesita permitir conexiones entrantes TCP 42861.
Para Internet, úsalo en un VPS con IP pública, o redirige TCP 42861 en el router
del servidor. Los jugadores no necesitan abrir puertos.

Si corre en tu mismo PC, usa 127.0.0.1:42861 desde ese PC. Para otros equipos
de la misma red, usa la IP local de la máquina del servidor. Para amigos por
Internet, usa su IP pública o nombre DNS. 127.0.0.1 no conecta PCs distintos.

## 2. Crear una partida

1. Abre ONLINE, CREATE ROOM.
2. Escribe el servidor en RELAY SERVER, por ejemplo 192.168.1.50:42861 para
   una LAN. Un nombre sin puerto usa 42861.
3. Elige modo, plazas y PUBLIC o PRIVATE.
4. Pulsa CREATE ROOM y espera ROOM ONLINE, TCP RELAY.
5. Si es pública, tus amigos la encuentran en SERVERS. Si es privada, pulsa
   COPY INVITE y comparte la invitación completa.
6. Espera a que estén listos y pulsa LAUNCH.

La sala aparece con el callsign del anfitrión. El anfitrión ejecuta la
simulación y debe mantener el juego abierto.

## 3. Encontrar y entrar

1. En SERVERS escribe la misma dirección en RELAY SERVER y pulsa REFRESH.
2. Verás las salas, modo, jugadores y plazas.
3. ALL MODES permite filtrar. PREV y NEXT recorren las páginas.
4. Selecciona una sala con JOIN. FULL significa que está llena.
5. En el lobby pulsa READY / WAIT para quedar listo.
6. El anfitrión pulsa LAUNCH cuando haya dos o más pilotos y todos estén
   listos. El anfitrión empieza listo.

La lista se actualiza cada tres segundos. Una partida en curso deja de
publicarse. Si falla la conexión, aparece el motivo y REFRESH reintenta.

Para una invitación privada: abre JOIN INVITE, selecciona el campo, pega con
Ctrl+V, pulsa Enter y luego JOIN ROOM. La invitación ya incluye el servidor.

Seleccionar un campo permite reemplazar su contenido escribiendo o pegando.
Ctrl+A selecciona todo, Backspace borra y Esc cancela la edición.

## 4. Voz y salida

El micrófono empieza apagado. En el lobby selecciona PUSH TO TALK o VOICE
DETECTION para activarlo. Push to talk usa U, configurable, o View/Back en el
mando. Hay mute, deafen y volumen por piloto.

Durante el vuelo Esc abre configuración y ONLINE / VOICE muestra el panel.
RESUME vuelve al vuelo. LEAVE ROOM desconecta. Abrir configuración no detiene
la partida compartida.

## Alternativa sin servidor: LAN DIRECT

En una red local sin relay, abre LAN DIRECT, elige UDP 42860 y HOST LAN ROOM.
Comparte COPY INVITE; los demás entran por JOIN INVITE. El firewall del
anfitrión debe permitir recibir UDP en la red privada. Estas salas no aparecen
en SERVERS. Para Internet usa el relay.

## Solución de problemas

| Síntoma | Qué revisar |
| --- | --- |
| SERVER UNREACHABLE o conexión rechazada | viper-server abierto, dirección correcta y TCP 42861 accesible. |
| Servidor online sin salas | Misma dirección, ALL MODES y una sala PUBLIC que aún no haya iniciado el vuelo. |
| ROOM FULL | Elige otra sala o pide al anfitrión más plazas. |
| No deja lanzar | Dos o más pilotos y todos en READY. |
| Se pierde la conexión | Anfitrión y relay deben seguir abiertos. Vuelve al navegador y reconecta. |
| Aviso de firewall | El juego por relay solo conecta hacia fuera. El servidor y LAN DIRECT sí reciben conexiones. |
| SmartScreen o antivirus | Es otro tipo de aviso. Los ejecutables aún no tienen firma Authenticode; no se puede garantizar su reputación desde el código. Guarda el mensaje exacto para revisar un posible falso positivo. |

No necesitas desactivar Defender ni añadir exclusiones. El juego no modifica
la seguridad de Windows. Una política que bloquee conexiones salientes también
puede impedir usar el relay.

La carpeta dist incluye SHA256SUMS.txt para verificar los archivos. El análisis
local se puede repetir con tools/check_windows_defender.py desde el repositorio;
genera un informe con hashes sin cambiar la configuración de seguridad.

Detalles técnicos: [MULTIPLAYER.md](MULTIPLAYER.md).
