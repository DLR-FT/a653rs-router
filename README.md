# IO-Partition

This is a prototype of an IO-partition for an ARINC 653P4 compliant hypervisor.
The project goal is to explore the possiblilties for network-transparent
inter-partition APEX channels in the context of redundancy management and
dynamic reconfiguration of IMA systems. It is developed in the memory-safe
Rust programming language and uses [a653rs](https://github.com/DLR-FT/a653rs)
to communicate with the hypervisor.

Keep in mind that this is a research project and it has not been developed and
tested according to DO-178C.

## Development

The development environment of this project is managed using [nix](https://nixos.org/download.html#download-nix).
To enter the environment, run `nix develop`.

## Documentation

```
cargo doc --all --open
```

## Nightly features

The echo example and support for Zynq7000 currently require a nightly compiler for support of const-generics.
Other than that all crates should use only stable rust features. 

## Flashing the XNG image

Building and flashing the FPGA image requires Vitis. Since the download is around 80 GiB large, it is not included in the devshell of this project.
Download it yourself and use the following shell to install it.

```
$ nix develop --no-write-lock-file github:nix-community/nix-environments#xilinx-vitis
```

After installation, close fhs environment shell, open a new one and try to run Vitis.
The UART examples use a specific configuration for the FPGA contained in [another project](https://gitlab.dlr.de/projekt-resilienz/vivado-coraz7-uart) which you need to compile and export.

```
$ source /opt/xilnx/Vitis/*/settings64.sh
$ vivado -nolog -nojournal -mode batch -source vivado_all.tcl uart.xpr
```

The resulting `hw_export.xsa` is needed for flashing along with an XNG image.

```
$ ./a653rs-router-zynq7000/flash "210370AD5202A" sys_img.elf hw_export.xsa
```
