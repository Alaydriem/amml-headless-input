name = "press"

# Start in a world with no screen open. Opens the inventory, waits, then closes it.
step "press" {
  key = "E"
}

step "wait" {
  for = "1s"
}

step "press" {
  key = "Esc"
}
