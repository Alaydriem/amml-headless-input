name = "walk-to"

defaults {
  goal_timeout = "20s"
}

# Edit the coordinates to a clear stretch of ground near your start point.
step "walk_to" {
  x      = 10.5
  z      = 0.5
  within = 0.5
}

step "walk_to" {
  x       = 10.5
  z       = 10.5
  sprint  = true
  timeout = "30s"
}
