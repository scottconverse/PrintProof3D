; Unsafe Out-of-bounds G-code print test
M140 S60
M104 S215
M190 S60
M109 S215
G28
G1 Z10 F3000
G1 X10 Y10 F6000
G1 X300 Y10 E5.0 F1500 ; UNSAFE: X=300 exceeds standard 220mm print bed!
G1 X300 Y300 E10.0 ; UNSAFE: Y=300 exceeds standard 220mm print bed!
M104 S0
M140 S0
G28 X0 Y0
M84
