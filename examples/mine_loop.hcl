name = "mine-loop"

# Face a block, break it for 2 s, turn 90 degrees. Five times.
repeat {
  count = 5

  step "attack" {
    hold = "2s"
  }

  step "turn" {
    yaw  = 90
    over = "10t"
  }
}
