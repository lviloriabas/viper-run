# Adaptive AoA protection

The FLCS retains the existing load-factor limits and now shares a wing-flow
budget between pitch, roll and pilot rudder input. This is a protection law for
the game aerodynamics, not a reproduction of the real aircraft's FLCS gains.

| Airframe | Nominal positive AoA cap | Positive g limit |
| --- | --- | --- |
| F-16C Block 50 | 25.5 degrees | 9 g |
| MiG-29S 9.13S | 25 degrees | 9 g below 14,200 kg and Mach 0.85; 8 g above that weight; 7 g above Mach 0.85 |

The negative load limit remains -3 g. The MiG's nominal AoA and existing
weight/Mach g schedule are unchanged. Real MiG-29 limits depend on the variant
and leading-edge flap position; a blanket increase to 26 degrees was not made.

## Wing-flow budget

Every integration substep, the computer starts from the same critical-AoA
calculation used by panel separation: Mach, altitude, ground proximity and
sideslip. It takes no credit for the temporary delay in separation during a
pitch-up. It subtracts the local AoA increments from current roll and sideslip,
the bound of the low-level gust model, and a 1.25 degree response reserve.
The lower of this budget and the airframe's nominal cap becomes the pitch
limit. Its onset moves down with it, preserving a 2.5 degree blending range.

The pitch-rate guard measures rotation of the airflow and anticipates AoA
motion by 0.15 seconds. This accounts for thrust and roll/yaw coupling rather
than using only a small-angle g/speed approximation, which previously allowed
the slow jet to settle above its nominal cap.

Roll rate is limited by the remaining local-AoA budget and by the nose-down
moment available to counter roll/yaw coupling. Half of the maximum steady
nose-down moment is reserved for pitch control, with allowance for the yaw
rate of a coordinated roll. Pilot pedal input
fades out as the worst-wing margin decreases from six to two degrees; the
yaw damper, sideslip feedback and aileron-rudder interconnect remain active.
Pilot pedals are also reduced during rapid rolling at high AoA.
Unloading restores lateral authority. The existing AOA LIMITER message also
reports protection of lateral commands. Available g uses lift at the adaptive
cap, with wing damage, and remains capped by the structural g limit. The AI
terrain-recovery prediction uses that available g with a response reserve,
so it does not assume a fixed six-g pull-up when the wing cannot supply it.

The limiter override bypasses the adaptive pitch, roll and pilot-pedal
protection. Separation, buffet, structural damage and departures remain in
the flight model. An overridden jet can still stall and exceed its g limit.
Protection cannot guarantee recovery from an already departed, damaged or
energy-depleted state; transonic buffet can occur before wing stall.

## Validation

Regression tests exercise both airframes in sustained pulls, abrupt pedal and
roll commands in both directions, combined controls, high-altitude transonic
flight, low-level gusts, release and return of lateral authority. Separate
checks retain loss of control with the override and recovery with nose-down
input after restoring protection. Existing positive/negative g and coordinated-roll tests remain.

## Real-aircraft references

- [USAF F-16 fact sheet](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104505/f-16-fighting-falcon/): nine-g capability with full internal fuel.
- [USAF comparison of F-16 and F/A-22](https://www.af.mil/News/Article-Display/Article/137832/cadets-take-controls-of-tomorrows-fighter/): 25.5 degree F-16 AoA reference.
- [NASA F-16 envelope expansion, conference publication](https://ntrs.nasa.gov/api/citations/19950007825/downloads/19950007825.pdf): baseline Category I envelope around 25 degrees; thrust vectoring was used to expand it.
- [GAF T.O. 1F-MIG29-1](https://www.airwar.ru/other/manuals/%5Baviation%5D%20-%20%5Bmanuals%5D%20-%20MiG-29%20Flight%20Manual.pdf): leading-edge flap switches select the higher 26 degree AoA limit in the German single-seat variant, versus the lower limit with flaps retracted.

The game's separation boundary is an approximation added to the aerodynamic
tables. These tests validate the game controller against that model; they do
not establish a real aircraft's flight envelope.
