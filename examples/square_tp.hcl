name = "square-tp"

# Start from a known pose, then walk a square of about 5 blocks a side. The
# command step sends one slash command to the server as the local player. Most
# servers need operator permission for a command like /tp. The wait gives the
# server time to move the player before the walk starts.

step "command" {
  text = "/tp @s ~ ~ ~ 90 0"
}

step "wait" {
  for = "10t"
}

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
