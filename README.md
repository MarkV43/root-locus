# RustLab

This is a library meant to, in the long run, replace MatLab with a faster (or blazingly fast), better to use, tool, with the same capabilities.

For now, we only have a Root Locus editor. To run it, clone the repository, and at its base directory, run 
```cargo run --release```
, and you should be good to go.

## Usage

Inside it, you can press `M` to change modes. There are currently 3 modes: Zoom, Interval and Precision. They change what the mouse wheel does. In Zoom mode, it zooms the plot in and out. You can also drag the plot with the mouse's middle button. In Interval mode, it changes the interval between the plot points, that is: spaces more or less the points. In Precision mode, it changes the precision with which the solver solves the roots to (that's why increasing it to a large enough values makes the plot wanky). Pressing `R` will fit all plot points in the screen. Pressing `F` will toggle FPS.