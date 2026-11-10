name = "join"

# Start at the main menu. A server must listen on 127.0.0.1:19132.
step "join_server" {
  address = "127.0.0.1"
  port    = 19132
  timeout = "60s"
}

step "walk_to" {
  x       = 5.5
  z       = 5.5
  timeout = "20s"
}

step "leave_world" {
  timeout = "30s"
}
