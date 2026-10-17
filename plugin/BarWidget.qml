import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris
import qs.Ui
import qs.Commons
import "Model.js" as Model

BarWidget {
  id: root
  moduleName: "yamarchy"

  readonly property var yamarchyPlayer: {
    var players = Mpris.players ? Mpris.players.values : []
    for (var i = 0; i < players.length; i++) {
      var p = players[i]
      if (p && (p.identity.indexOf("Yamarchy") !== -1 || p.desktopEntry === "yamarchy" || p.identity.indexOf("yamarchy") !== -1)) {
        return p
      }
    }
    return null
  }

  readonly property bool isPlaying: yamarchyPlayer ? yamarchyPlayer.isPlaying : false
  readonly property string trackTitle: yamarchyPlayer ? (yamarchyPlayer.trackTitle || "") : ""
  readonly property string trackArtist: yamarchyPlayer ? (yamarchyPlayer.trackArtist || "") : ""
  readonly property real playerVolume: yamarchyPlayer ? (yamarchyPlayer.volume || 0.8) : 0.8

  function sendCli(args) {
    cliProcess.command = ["yamarchy"].concat(args)
    cliProcess.running = true
  }

  function togglePlay() {
    if (yamarchyPlayer) {
      yamarchyPlayer.playPause()
    } else {
      sendCli(["toggle"])
    }
  }

  readonly property bool opened: panelLoader.item ? panelLoader.item.opened === true : false

  function open() {
    if (panelLoader.item) panelLoader.item.open()
  }

  function close() {
    if (panelLoader.item) panelLoader.item.close()
  }

  function togglePanel() {
    if (panelLoader.item) panelLoader.item.toggle()
  }

  function injectPanel() {
    var target = panelLoader.item
    if (!target) return
    if ("bar" in target) target.bar = root.bar
    if ("anchorItem" in target) target.anchorItem = button
    if ("hostWidget" in target) target.hostWidget = root
  }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  onBarChanged: injectPanel()

  Process {
    id: cliProcess
    running: false
  }

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    slotSize: Style.bar.iconSlot
    tooltipText: root.trackTitle !== "" ? (root.trackTitle + (root.trackArtist !== "" ? " — " + root.trackArtist : "")) : "Yandex Music"

    iconComponent: Component {
      Item {
        anchors.fill: parent
        YandexMusicIcon {
          anchors.centerIn: parent
          iconSize: Style.space(18)
          color: root.isPlaying ? (root.bar ? root.bar.foreground : Color.accent) : (root.bar ? root.bar.foreground : Color.foreground)
          opacity: root.isPlaying ? 1.0 : 0.65
        }
      }
    }

    onPressed: function(b) {
      if (b === Qt.RightButton) root.togglePlay()
      else root.togglePanel()
    }

    onWheelMoved: function(delta) {
      var steps = delta > 0 ? 5 : -5
      var newVol = Math.max(0, Math.min(100, Math.round(root.playerVolume * 100) + steps))
      root.sendCli(["volume", newVol.toString()])
    }
  }

  Loader {
    id: panelLoader
    active: true
    source: Qt.resolvedUrl("Panel.qml")
    visible: false
    onLoaded: {
      root.injectPanel()
      Qt.callLater(root.injectPanel)
    }
  }

  IpcHandler {
    target: "yamarchy"

    function open(): void { root.open() }
    function close(): void { root.close() }
    function toggle(): void { root.togglePanel() }
    function play(): void { root.sendCli(["play"]) }
    function pause(): void { root.sendCli(["pause"]) }
    function next(): void { root.sendCli(["next"]) }
    function prev(): void { root.sendCli(["prev"]) }
    function vibe(station: string): void { root.sendCli(["vibe", station || "user:onyourwave"]) }
    function vibeSet(diversity: string, mood: string, language: string): void {
      var args = ["vibe-set"]
      if (diversity) args = args.concat(["--diversity", diversity])
      if (mood) args = args.concat(["--mood", mood])
      if (language) args = args.concat(["--language", language])
      root.sendCli(args)
    }
    function crossfade(state: string, duration: int, onSkip: string): void {
      var args = ["crossfade"]
      if (state) args = args.concat([state])
      if (duration > 0) args = args.concat(["--duration", duration.toString()])
      if (onSkip) args = args.concat(["--on-skip", onSkip])
      root.sendCli(args)
    }
  }
}
