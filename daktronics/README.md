# Daktronics AF-6700 variant signs
These signs were made by Daktronics for NextBus (now part of
Cubic Transportation Systems) and are/were used to display transit arrival times
to passengers in bus shelters or on platforms in many transit systems.
The ones under study here were acquired from Portland Streetcar, which in 2025
had retired all but one of their Daktronics/NextBus signs as part of a switch to
ConnectPoint for passenger information displays (to harmonize with TriMet's
hardware).

Inside each sign there are two LED matrix panels,
a brightness control board,
an LED matrix controller,
a single-board computer (SBC),
a cellular modem and antenna,
a power supply, and
a watchdog timer board.
In operation the SBC communicates with a central server via the modem and uses
the

Public documentation on these is scant; accordingly, efforts are underway to
reverse-engineer the [LED matrix panels](led-matrices.md),
[mechanical specifications](mechanical.md),
[single-board computer](sbc.md), and the
[matrix controller](controller.md).
Present efforts involve bypassing all of the existing hardware except for the
power supply, LED matrix panels, and antenna,
and substituting a new matrix controller.
There exists a design for an
[ESP32 dev board-based matrix controller](matrix-controller-esp32/README.md),
while efforts are underway to design a cost-reduced ESP32-based matrix
controller that would use a custom PCB and associated electronics,
and would be able to take advantage of the built-in external antenna (which is
sometimes, depending on the particular model of antenna installed in each sign,
capable of communication on the 2.4 GHz ISM bands used for WiFi and Bluetooth).
