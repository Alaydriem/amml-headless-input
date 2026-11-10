name = "square"

# Walk a square of about 5 blocks a side. At the walk speed, 1.1 s is
# roughly 5 blocks. Each turn is 90 degrees over 10 ticks.

repeat {
  count = 4

  step "move" {
    hold = ["forward"]
    for  = "1.1s"
  }

  step "turn" {
    yaw  = 90
    over = "10t"
  }
}
