The LED matrix assembly contains two LED matrix panels side-by-side,
a brightness control board, a light-dependent resistor (LDR) board,
and power/data wiring.

# Panels
Each LED matrix panel contains a 16×48 LED matrix PCB, which is composed of two
independent
8×48 matrix quadrants combined on one PCB.

The panel PCBs are connected from bottom to top, right to left (from the front).
In terms of matrix topology and data flow, they function together
as one long 8×192 matrix.

## IC and semiconductor list (per quadrant)
- 3× [74HCT541D](https://assets.nexperia.com/documents/data-sheet/74HC_HCT541.pdf)
octal line driver
- 6× [MB15169GDW](https://www.neumueller.com/datenblatt/macroblock/MBI5169%20Datenblatt%20-%20Datasheet.pdf)
shift register-based LED driver
- 1× [HEF4028BT](https://assets.nexperia.com/documents/data-sheet/HEF4028B.pdf)
4-bit binary to 10-line decimal decoder
- 1× [TD62784AFG](https://docs.rs-online.com/bc35/0900766b80811071.pdf)
high-voltage source driver
- 8× [RD16N05](https://www.mouser.com/datasheet/2/149/RFD16N05SM-98571.pdf)
N-channel power MOSFET

## Column drive arrangement
Each column is driven by one of the output lines of the six MBI5169 chips.
Each chip drives eight columns.
These chips act like shift registers; in this way, data are shifted into each
quadrant, through each column, and then out to the next quadrant.

## Row drive arrangement
Each row is switched through a single RD16N05 power MOSFET.
These MOSFETs are driven by the TD62784AFG driver chip.
This chip in turn gets its input from the HEF4028 BCD decoder;
a 0 input to this decoder results in the top row being enabled.
In this way, only one row is lit at a time.

## Pertinent signals
### Column drive (CD)
| Short name | Function                |
|------------|-------------------------|
| CD CLK     | Shift register clock |
| CD SI      | Serial data input |
| CD LE/MOD  | Latch shift register contents; a high pulse on this line causes the shift register contents to be transfered to the output latch. Also used for mode switching. |
| CD style="text-decoration:overline">OE</span>/SW/<span style="text-decoration:overline">ED</span> | Active-low output enable signal. Also used for mode switching. |
The column drive ICs are capable of detecting open/short LEDs, and can be
switched into an error detection mode to facilitate this.
This is not implemented by the original controller,
and utilizing this feature would
be rather difficult given how the matrix panels are designed.
Interested readers are referred to the
[MB15169GDW datasheet](https://www.neumueller.com/datenblatt/macroblock/MBI5169%20Datenblatt%20-%20Datasheet.pdf)
for more information.

### Row drive (RD)
| Short name | Function                |
|------------|-------------------------|
| RD A0      | Bit 0 for row selection |
| RD A1      | Bit 1 for row selection |
| RD A2      | Bit 2 for row selection |
| RD A3      | Bit 3 for row selection |

## Cable pinouts
### Power connector (PL3)
| Pin | Function |
|-----|----------|
| 1   | 3.3 V    |
| 2   | 5 V      |
| 3   | 0 V      |
### Matrix in/out connector (PL1/PL2)
| Pin | Short name | Pulldown? | Topo notes                                            | Remarks                                        |
|-----|------------|-----------|-------------------------------------------------------|------------------------------------------------|
|  1  | CD CLK     |       Yes | IC1 ?/A? =>                                           |                                                |
|  2  | GND        |       N/A |                                                       | Signal ground                                  |
|  3  | CD SI      |       Yes |                                                       | |
|  4  | GND        |       N/A |                                                       | Signal ground                                  |
|  5  | RD A0      |       Yes |                                                       | |
|  6  | GND        |       N/A |                                                       | Signal ground                                  |
|  7  | RD A1      |       Yes |                                                       | |
|  8  | N/C        |           | DNP'd JMP LK1, p 3 of DNP'd PL4, + term of DNP'd C110 | N/C (+12V on control board)  |
|  9  | RD A2      |       Yes |                                                       | |
| 10  | GMD        |       N/A |                                                       | Signal ground                                  |
| 11  | RD A3      |       Yes |                                                       | |
| 12  | GMD        |       N/A |                                                       | Signal ground                                  |
| 13  | CD LE/MOD  |       Yes |                                                       | |
| 14  | GMD        |       N/A |                                                       | Signal ground                                  |
| 15  | CD <span style="text-decoration:overline">OE</span>/SW/<span style="text-decoration:overline">ED</span> | Mo | IC1 9/A7 ⇒ IC1 11/Y7 → IC6/10/12/14/16 13/!OE/SW/!ED | Output enable inv/Mode switch/Error detect inv |
| 16  | GMD        |       N/A |                                                       | Signal ground                                  |
| 17  | N/C        |           |                                                       | ? (N/C on control board)                       |
| 18  | GMD        |       N/A |                                                       | Signal ground                                  |
| 19  | N/C        |           |                                                       | ? (N/C on control board)                       |
| 20  | GMD        |       N/A |                                                       | Signal ground                                  |
