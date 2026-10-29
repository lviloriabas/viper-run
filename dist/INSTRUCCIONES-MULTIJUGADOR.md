# Multijugador, instrucciones de uso

## Inicio

Todos deben usar el juego actualizado. Basta con abrir el ejecutable del juego:
no hay que abrir viper-server, descargar otro programa ni usar una consola.
Abre ONLINE y elige CALLSIGN y avión. El menú ofrece CREATE GAME, JOIN FRIEND,
NEARBY y ADVANCED.

## 1. Crear una partida desde el juego

1. Abre ONLINE, CREATE GAME. Deja HOST ON THIS PC, que es el valor inicial.
2. Elige modo y plazas. PUBLIC permite que aparezca en NEARBY; INVITE ONLY
   requiere una invitación.
3. Pulsa CREATE GAME. El juego inicia y mantiene su propio servidor.
4. Si Windows lo pregunta, permite al juego recibir conexiones en tu red privada.
5. Mantén el juego abierto. Al pulsar LEAVE ROOM se cierra también el servidor.

La sala aparece con el callsign del anfitrión. El anfitrión ejecuta la
simulación y debe mantener el juego abierto.

## 2. Conectarse en la misma red (Wi-Fi o cable)

1. El segundo jugador abre ONLINE, NEARBY. La búsqueda empieza sola.
2. Selecciona la partida de tu amigo con JOIN.
3. El invitado pulsa READY / WAIT y el anfitrión pulsa LAUNCH.

Si la partida no aparece, el anfitrión pulsa COPY LAN INVITE y comparte el
texto completo. El invitado lo copia, abre JOIN FRIEND y pulsa PASTE AND JOIN.
También puede pegar en el campo y pulsar Enter para conectarse directamente.
No hay que escribir IP ni puerto. Una red de invitados con aislamiento entre
equipos puede bloquear tanto la búsqueda como la conexión.

La invitación es un texto generado por el juego con el formato
`relay+tcp:IP:PUERTO#CLAVE_DE_LA_SALA`. Los botones COPY generan el texto real
completo. El anfitrión comparte todo ese texto, no solo la IP ni el puerto;
el invitado no tiene que construir ni editar la clave.

## 3. Conectarse por Internet

Al crear la partida, el juego intenta obtener una conexión temporal en el
router mediante UPnP, en segundo plano. Cuando está disponible aparece COPY
INTERNET INVITE. Comparte esa invitación; tu amigo abre JOIN FRIEND y pulsa
PASTE AND JOIN. El invitado marca READY y el anfitrión pulsa LAUNCH.

COPY LAN INVITE es solo para la misma red. No sirve para conectar por Internet.
La apertura automática depende del router y no es una comprobación desde otra
red: el firewall o un segundo router todavía pueden impedir la conexión. Si
falla, prueba el otro PC como anfitrión y consulta INTERNET HELP dentro del juego.

### Si hay que abrir el puerto manualmente

En el lobby pulsa CONNECTION HELP / PORTS. El juego muestra la IP local y el
puerto TCP exactos de esa sala; normalmente usa 42861, pero puede elegir otro
si está ocupado. COPY ROUTER STEPS copia instrucciones con los valores reales.

1. En tu router busca Port Forwarding, Redirección de puertos o Virtual Server.
2. Crea una regla TCP con el puerto externo mostrado, la IP local del PC que
   muestra el juego y ese mismo puerto como puerto interno. Guarda o aplica.
   No necesitas abrir UDP para esta modalidad. Solo lo hace el anfitrión.
3. Permite al juego recibir conexiones en la red privada de Windows si aparece
   la solicitud. Mantén el juego abierto.
4. Busca la IPv4 WAN / Internet de tu router y escríbela en PUBLIC IPV4 / DNS.
   Si el router la comunica al juego, el campo se completa automáticamente.
   También se acepta tu nombre DNS. Si elegiste un puerto externo diferente,
   escribe `IP_PUBLICA:PUERTO_EXTERNO`.
5. Pulsa COPY INTERNET INVITE en esa pantalla. El juego añade automáticamente
   el puerto y la clave. Comparte la invitación completa; tu amigo pulsa
   PASTE AND JOIN.

La IP pública es distinta de direcciones locales como 192.168.x.x. Una WAN
privada (10.x.x.x, 192.168.x.x o 172.16 a 172.31) o de CGNAT (100.64 a 100.127)
requiere resolver también el router/proveedor anterior; abrir solo este puerto
no basta. La ayuda muestra alternativas. Generar la invitación manual no prueba
que el puerto sea accesible desde otra red.

Si el router no acepta UPnP o el proveedor usa CGNAT, el juego lo indica cuando
puede detectarlo. La LAN sigue disponible. Para esas conexiones de Internet se
puede usar un relay externo accesible: en CREATE GAME pulsa CHANGE para elegir
USE EXTERNAL RELAY y escribe su dirección. ADVANCED permite navegar por sus
salas públicas. El relay solo necesita configurarse una vez y su dirección se
recuerda. No se incluye un servicio público desplegado automáticamente.

Las conexiones automáticas del router tienen una duración limitada, se renuevan
mientras se juega y el juego solicita retirarlas al cerrar la sala. No modifica
las reglas de Defender ni el firewall de Windows.

Seleccionar un campo permite reemplazar su contenido escribiendo o pegando.
Ctrl+A selecciona todo, Backspace borra y Esc cancela la edición.

## 4. Voz y salida

El micrófono empieza apagado. En el lobby selecciona PUSH TO TALK o VOICE
DETECTION para activarlo. Push to talk usa U, configurable, o View/Back en el
mando. Hay mute, deafen y volumen por piloto.

Durante el vuelo Esc abre configuración y ONLINE / VOICE muestra el panel.
RESUME vuelve al vuelo. LEAVE ROOM desconecta. Abrir configuración no detiene
la partida compartida.

## Relay externo opcional

Para administrar un relay dedicado sigue disponible viper-server. En Windows
abre viper-server-windows-x86_64.exe; en Linux ejecuta
`./dist/viper-server-linux-x86_64 0.0.0.0:42861`. Solo esa máquina necesita TCP
42861 entrante. El juego normal no requiere abrir este programa.

## Solución de problemas

| Síntoma | Qué revisar |
| --- | --- |
| SERVER UNREACHABLE o conexión rechazada | viper-server abierto, dirección correcta y TCP 42861 accesible. |
| NEARBY no encuentra la sala | Misma red, sala PUBLIC, permitir el juego en la red privada del anfitrión; probar COPY LAN INVITE. |
| No aparece COPY INTERNET INVITE | CONNECTION HELP / PORTS muestra la IP y el puerto para abrirlo manualmente y generar la invitación. Si hay CGNAT, probar el otro PC o un relay externo. |
| Servidor externo online sin salas | Misma dirección, ALL MODES y una sala PUBLIC que aún no haya iniciado el vuelo. |
| ROOM FULL | Elige otra sala o pide al anfitrión más plazas. |
| No deja lanzar | Dos o más pilotos y todos en READY. |
| Se pierde la conexión | El anfitrión debe mantener el juego abierto. Pedir una invitación de la sala actual. |
| Aviso de firewall | Al alojar en este PC, permite el juego en la red privada. Al usar un relay externo, los jugadores solo conectan hacia fuera. |
| SmartScreen o antivirus | Es otro tipo de aviso. Los ejecutables aún no tienen firma Authenticode; no se puede garantizar su reputación desde el código. Guarda el mensaje exacto para revisar un posible falso positivo. |

No necesitas desactivar Defender ni añadir exclusiones. El juego no modifica
la seguridad de Windows. Una política que bloquee conexiones salientes también
puede impedir usar el relay.

La carpeta dist incluye SHA256SUMS.txt para verificar los archivos. El análisis
local se puede repetir con tools/check_windows_defender.py desde el repositorio;
genera un informe con hashes sin cambiar la configuración de seguridad.

Detalles técnicos: [MULTIPLAYER.md](MULTIPLAYER.md).
