; Unsafe Temperature G-code print test
M140 S60
M104 S350 ; UNSAFE: Nozzle set temp to 350C (exceeds typical 300C max limit!)
M190 S60
M109 S350 ; Wait for unsafe nozzle temp
G28
G1 Z10 F3000
M104 S0
M140 S0
M84
