Webloom reference-only clock adaptation, applied after the pinned Web2C chain.
Restore the deterministic date of pristine tex.web section 241. Keep the C
call because it also installs the interrupt handler; discard its clock values.
This file does not modify the normative WEB or the upstream checkout.

@x
begin date_and_time(sys_time,sys_day,sys_month,sys_year);
@y
begin date_and_time(sys_time,sys_day,sys_month,sys_year);
sys_time:=12*60;
sys_day:=4; sys_month:=7; sys_year:=1776;
@z
