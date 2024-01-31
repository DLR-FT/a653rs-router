#!/usr/bin/env xsct

set region [lindex $argv 0]
set cable [lindex $argv 1]
set cfg [lindex $argv 2]

connect

targets -set -nocase -filter { name =~ "APU*" && jtag_cable_serial == $cable };

mwr -bin -file $cfg $region
