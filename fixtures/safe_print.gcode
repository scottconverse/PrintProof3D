; Safe G-code print test
M140 S60 ; Set bed temperature to 60C
M104 S215 ; Set hotend temperature to 215C
M190 S60 ; Wait for bed to reach 60C
M109 S215 ; Wait for hotend to reach 215C
G28 ; Home all axes
G1 Z10 F3000 ; Move nozzle up to Z=10
G1 X10 Y10 F6000 ; Move to safe X Y coordinate
G1 X100 Y10 E5.0 F1500 ; Extrude a line to X=100
G1 X100 Y100 E10.0 ; Extrude line to Y=100
M104 S0 ; Turn off hotend
M140 S0 ; Turn off bed
G28 X0 Y0 ; Home X and Y
M84 ; Disable steppers
