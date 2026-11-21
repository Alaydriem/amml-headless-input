namespace    = "alaydriem"
name         = "headless-input"
entry        = "alaydriem_headless_input.dll"
api_version  = 3

uses {
  feature "core" {
    event "client.tick" {
      version  = 1
      required = true
    }
    event "client.game_tick" {
      version  = 2
      required = true
    }
    event "world.left" {
      version  = 1
      required = true
    }
    event "registry.provider_ready" {
      version  = 1
      required = true
    }
    event "registry.provider_gone" {
      version  = 1
      required = true
    }
    event "mods.loaded" {
      version  = 1
      required = true
    }
    area "client.read" {
      version  = 1
      required = true
    }
    area "client.top_screen" {
      version  = 2
      required = true
    }
  }
  feature "input" {
    event "input.lease_suspended" {
      version  = 1
      required = true
    }
    event "input.lease_resumed" {
      version  = 1
      required = true
    }
    event "input.lease_lost" {
      version  = 1
      required = true
    }
    event "input.injected_released" {
      version  = 2
      required = true
    }
    area "input.lease" {
      version  = 1
      required = true
    }
    area "input.move" {
      version  = 1
      required = true
    }
    area "input.key_inject" {
      version  = 1
      required = true
    }
    area "input.mouse_inject" {
      version  = 1
      required = true
    }
    area "input.turn" {
      version  = 1
      required = true
    }
  }
  feature "player" {
    area "actor.read" {
      version  = 2
      required = true
    }
  }
  feature "hotbar" {
    area "hud.select_hotbar" {
      version  = 1
      required = true
    }
  }
  feature "join" {
    event "client.join_server_result" {
      version  = 1
      required = true
    }
    area "client.join_server" {
      version  = 1
      required = true
    }
    area "client.can_join" {
      version  = 1
      required = true
    }
    area "client.leave_world" {
      version  = 1
      required = true
    }
    area "client.start_local_world" {
      version  = 1
      required = true
    }
    area "client.open_menu" {
      version  = 1
      required = true
    }
  }
  feature "command" {
    area "client.send_command" {
      version  = 1
      required = true
    }
  }
  feature "screenshot" {
    event "overlay.screenshot_taken" {
      version  = 1
      required = true
    }
    event "overlay.screenshot_written" {
      version  = 1
      required = true
    }
    area "overlay.screenshot" {
      version  = 1
      required = true
    }
  }
}

listing {
  title   = "Headless Input"
  summary = "Drives the local player with injected input, from live MCP calls or a control file."
  authors = ["Charles R. Portwood II"]
  license = "BSD-3-Clause"
  source  = "https://github.com/alaydriem/amml-headless-input"
  homepage = "https://github.com/alaydriem/amml-headless-input"
  tags    = ["input", "automation", "mcp", "client"]
  icon    = "icon.png"
}
