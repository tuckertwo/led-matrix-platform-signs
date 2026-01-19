overall_x = 155;
mountbar_z = 15;

clearance = 52;

foot_x = 50;
foot_z = 15;

holepitch = 127;

$fn=100;
union()
{
  translate([0,50,foot_z+clearance-mountbar_z]) difference()
  {
    cube([overall_x, 42, mountbar_z]);
    for(x=[0:holepitch:holepitch]) translate([(overall_x-holepitch)/2+x,42/2,-0.01])
    {
      cylinder(r=5.45/2, h=100);
      cylinder(r=9/2, h=5);
    }
  }
  #translate([overall_x/2-foot_x/2, 50, 0]) cube([foot_x, 42,  clearance]);
  translate([overall_x/2-foot_x/2, 0, 0]) cube([foot_x, 200, foot_z]);
}
