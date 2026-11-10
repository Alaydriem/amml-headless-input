namespace    = "alaydriem"
name         = "headless-input"
entry        = "alaydriem_headless_input.dll"
api_version  = 2
amml_version = "26.60.6"
entries = [
  "ClientInstance.update",
  "ClientInstance.onTick",
  "ClientInstance.gameCallbacksOffset",
  "ClientInstance.getLocalPlayer",
  "ClientInstance.getTopScreenName",
  "ClientInstance.vtable",
  "Actor.stateVectorComponent",
  "StateVectorComponent.mPos",
  "Actor.actorRotationComponent",
  "ActorRotationComponent.mRot",
  "Actor.getDimension",
  "Dimension.getDimensionId",
  "ServerWorldJoiner.joinExternalServer",
  "MinecraftGame.getWorldSystem",
  "WorldSystem.mNetworkWorldJoiner",
  "NetworkWorldJoiner.mServerWorldJoiner",
  "ClientInstance.getMinecraftGame",
  "MinecraftGame.isPrimaryClientInstanceReady",
  "ClientInstance.requestLeaveGame",
  "ClientInstance.getCurrentUIRouter",
  "OreUIRouter.pushRoute",
  "MinecraftPackets.createPacket",
  "ClientInstance.getPacketSender",
  "PacketSender.send",
  "CommandRequestPacket.mCommand",
  "CommandRequestPacket.size",
]

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
