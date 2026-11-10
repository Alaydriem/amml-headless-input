namespace    = "alaydriem"
name         = "headless-input"
display_name = "Headless Input"
version      = "26.62.0"
author       = "Alaydriem"
config_path  = "config.hcl"

claim "Input" {
  mode       = "Exclusive"
  reason     = "Drives the player while a run or live session is active"
  full_block = true
}

section "mcp" {
  title = "MCP"

  field "mcp.enabled" {
    label   = "Serve MCP"
    help    = "Lets Claude Code drive the player through the MCP hub"
    kind    = "bool"
    default = false
    apply   = "Live"
  }

  field "mcp.allow_pairing" {
    label   = "Allow MCP pairing"
    help    = "Turn this on while Claude Code asks to pair. The mod turns it off again after the answer"
    kind    = "bool"
    default = false
    apply   = "Live"
  }

  field "mcp.port" {
    label   = "Hub port"
    help    = "The port of the MCP hub. The first instance to bind it serves every instance"
    kind    = { int = { min = 1024, max = 65535, step = 1 } }
    default = 28250
    apply   = "Live"
  }
}

section "run" {
  title = "Runs"

  field "run.goal_timeout" {
    label   = "Goal timeout"
    help    = "The longest a goal step may run before the run fails"
    kind    = { enum = { options = [
      { value = "10s", label = "10 seconds" },
      { value = "30s", label = "30 seconds" },
      { value = "60s", label = "60 seconds" },
      { value = "120s", label = "2 minutes" },
    ] } }
    default = "30s"
    apply   = "Live"
  }

  field "run.watchdog" {
    label   = "Watchdog"
    help    = "The longest a run may go without progress before it is stopped"
    kind    = { enum = { options = [
      { value = "2s", label = "2 seconds" },
      { value = "5s", label = "5 seconds" },
      { value = "10s", label = "10 seconds" },
      { value = "30s", label = "30 seconds" },
    ] } }
    default = "5s"
    apply   = "Live"
  }
}

section "live" {
  title = "Live sessions"

  field "live.idle" {
    label   = "Idle end"
    help    = "A live session ends after this long without a command"
    kind    = { enum = { options = [
      { value = "10s", label = "10 seconds" },
      { value = "30s", label = "30 seconds" },
      { value = "60s", label = "60 seconds" },
      { value = "300s", label = "5 minutes" },
    ] } }
    default = "30s"
    apply   = "Live"
  }
}

section "files" {
  title = "Control files"

  field "files.folder" {
    label   = "Folder"
    help    = "The folder of control files. The default is the mod data folder; set another folder in config.hcl"
    kind    = { enum = { options = [
      { value = "", label = "Mod data folder" },
    ] } }
    default = ""
    apply   = "Live"
  }
}
