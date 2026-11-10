name = "portal"

defaults {
  goal_timeout = "30s"
}

# Start in front of a lit portal. Walk in, wait in the Nether, walk back.
step "walk_until_dimension" {
  dimension = "nether"
  timeout   = "30s"
}

step "wait" {
  for = "5s"
}

step "walk_until_dimension" {
  dimension = "overworld"
  timeout   = "30s"
}
