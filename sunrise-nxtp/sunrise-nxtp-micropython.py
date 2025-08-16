from time import sleep
from machine import Pin, UART, I2C
from math import ceil
uart = UART(0, baudrate=9600, bits=8, parity=None, stop=1,
            tx=Pin.board.GP0, rx=Pin.board.GP1)
de = Pin(Pin.board.GP2, Pin.OUT)

class Sunrise():
    def __init__(self, signno=0, uart=uart, de=de):
        self.signno = signno
        self.uart = uart
        self.de = de


    def poweron_test(self):
        sleep(21)
        self.test()


    def test(self):
        self.messagetx(b'WRM Sunrise Test')
        sleep(1)
        for i in range(18):
            self.messagetx(b'^Y'+str(i))
            sleep(1)
        self.messagetx(b'Test complete.')


    def reset(self):
        self.packettx(b'M\x00\x01\x00')


    def pleaseholdon(self):
        for i in range(9):
            sun.messagetx(b'PLEASE HOLD ON')
            sleep(1.1)
            sun.messagetx(b'')
            sleep(0.7)


    def messagetx(self, message):
        assert len(message) <= 180
        for i in range(ceil(len(message)/12)):
            self.packettx(b'M\x00\x01'+bytes([(i+1)*16+1])
                +message[i*12:max(len(message), (i+1)*12)])
        self.packettx(b'T')


    def packettx(self, packetbody):
        data_nocheck = [195, 255, 245, len(packetbody)] + list(packetbody)
        self.datatx(data_nocheck)


    def datatx(self, data_nocheck):
        data = data_nocheck + [256-(sum(data_nocheck)%256)]
        self.de.value(1)
        sleep(0.01)
        self.uart.write(bytes(data))
        self.uart.flush()
        sleep(0.01)
        self.de.value(0)

sun = Sunrise()
sun.poweron_test()
