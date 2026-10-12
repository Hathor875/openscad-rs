// Compatible subset of the official linear_extrude example.
translate([0, -15, 0])
    linear_extrude(height = 20)
        square([20, 10], center = true);
