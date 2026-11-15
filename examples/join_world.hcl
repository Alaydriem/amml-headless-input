name = "join_world"

# Start at the main menu. A world named "Test Flat" must exist before the game starts.
step "join_world" {
  world   = "Test Flat"
  timeout = "120s"
}

step "walk_to" {
  x       = 5.5
  z       = 5.5
  timeout = "20s"
}

step "leave_world" {
  timeout = "30s"
}
