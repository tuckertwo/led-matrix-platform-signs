Each sign contains an
[EmbeddedTS TS-7260](https://www.embeddedts.com/products/TS-7260) single-board
computer. Documentation can be found [here](https://docs.embeddedts.com/TS-7260).

## Getting a Shell

To get a console over a serial port, note or set the position of jumper pin
\4. If set, the console output goes to COM2. The baud rate is 115200. Once
connected, press <kbd>Ctrl+C</kbd> to interrupt the boot and get a console.
Then, the following sequence of commands will get a shell:

```
fis load linux
exec -c "console=ttyAM0,115200 root=/dev/mtdblock1 init=/bin/sh"
```

## Networking

The DHCP client installed on the devices has been observed to fail to get a
lease on some networks. The init scripts are the simplest way to configure the
network is via `/etc/sysconfig/ifcfg-eth0`, which is sourced from
`/etc/init.d/network`.

## SSH

By default, root login over SSH is disabled. Modify `/etc/inetd.conf` to remove
`-s` from the Dropbear invocation if you'd like to do this.

Additionally, the Dropbear SSH server installed by default on the machine at port 2222 is
quite old, so the options in the SSH command line below are required:

```
ssh -p 2222 HostKeyAlgorithms=+ssh-rsa -o KexAlgorithms=+diffie-hellman-group1-sha1 -o MACs=hmac-sha1 -o Ciphers=+3des-cbc <USER>@<IP>
```
