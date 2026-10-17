import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris
import qs.Ui
import qs.Commons
import "Model.js" as Model

Panel {
  id: root
  moduleName: "yamarchy"
  ipcTarget: "yamarchy"
  manageIpc: false

  readonly property string iconFontFamily: "JetBrainsMono Nerd Font, monospace"

  property string currentView: "player" // "player" | "settings"
  property bool isPlaying: false
  property string trackTitle: "Yamarchy (Stopped)"
  property string trackArtist: "Select a station"
  property string trackAlbum: ""
  property string artUrl: ""
  property real trackLength: 0
  property real trackPosition: 0
  property real playerVolume: 0.8
  property bool isLiked: false
  property string currentStation: "user:onyourwave"
  property string currentDiversity: "default"
  property string currentMood: "all"
  property string currentLanguage: "any"
  property bool vibeExpanded: false

  // Audio / Crossfade settings
  property bool crossfadeEnabled: false
  property int crossfadeDuration: 3
  property bool crossfadeOnSkip: false

  onOpenedChanged: {
    if (!root.opened) {
      root.vibeExpanded = false
    }
  }

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

  function formatVibeSummary() {
    var d = root.currentDiversity
    var divLabel = "Favorite"
    if (d === "discover") divLabel = "Discover"
    else if (d === "popular") divLabel = "Popular"

    var m = root.currentMood
    var moodLabel = "All"
    if (m === "active") moodLabel = "Active"
    else if (m === "fun") moodLabel = "Fun"
    else if (m === "calm") moodLabel = "Calm"
    else if (m === "sad") moodLabel = "Sad"

    return divLabel + " • " + moodLabel
  }

  function applyState(data) {
    if (!data) return
    root.isPlaying = data.status === "playing"
    if (data.current_track) {
      root.trackTitle = data.current_track.title || "Unknown Title"
      root.trackArtist = data.current_track.artist || "Unknown Artist"
      root.trackAlbum = data.current_track.album || ""
      root.artUrl = data.current_track.cover_url || ""
      root.isLiked = data.current_track.liked === true
    } else {
      root.trackTitle = "Yamarchy (Stopped)"
      root.trackArtist = "Select a station"
      root.trackAlbum = ""
      root.artUrl = ""
      root.isLiked = false
    }
    root.trackPosition = (data.position_ms || 0) / 1000
    root.trackLength = (data.duration_ms || 0) / 1000
    root.playerVolume = data.volume !== undefined ? data.volume : 0.8

    if (data.station) root.currentStation = data.station
    if (data.vibe_settings) {
      root.currentDiversity = data.vibe_settings.diversity || "default"
      root.currentMood = data.vibe_settings.mood_energy || "all"
      root.currentLanguage = data.vibe_settings.language || "any"
    }
    if (data.audio_settings) {
      root.crossfadeEnabled = data.audio_settings.crossfade_enabled === true
      root.crossfadeDuration = data.audio_settings.crossfade_duration || 3
      root.crossfadeOnSkip = data.audio_settings.crossfade_on_manual_skip === true
    }
  }

  function sendCli(args) {
    cliProcess.command = ["yamarchy"].concat(args)
    cliProcess.running = true
  }

  function togglePlay() {
    sendCli(["toggle"])
  }

  function nextTrack() {
    sendCli(["next"])
  }

  function prevTrack() {
    sendCli(["prev"])
  }

  function toggleLike() {
    sendCli(["like"])
  }

  function dislikeTrack() {
    sendCli(["dislike"])
  }

  function playStation(station) {
    root.currentStation = station
    sendCli(["vibe", station || "user:onyourwave"])
  }

  function setDiversity(val) {
    root.currentDiversity = val
    sendCli(["vibe-set", "--diversity", val])
  }

  function setMood(val) {
    root.currentMood = val
    sendCli(["vibe-set", "--mood", val])
  }

  function setLanguage(val) {
    root.currentLanguage = val
    sendCli(["vibe-set", "--language", val])
  }

  function toggleCrossfade() {
    var next = !root.crossfadeEnabled
    root.crossfadeEnabled = next
    sendCli(["crossfade", next ? "on" : "off"])
  }

  function setCrossfadeDuration(dur) {
    root.crossfadeDuration = dur
    sendCli(["crossfade", "--duration", dur.toString()])
  }

  function toggleCrossfadeOnSkip() {
    var next = !root.crossfadeOnSkip
    root.crossfadeOnSkip = next
    sendCli(["crossfade", "--on-skip", next ? "on" : "off"])
  }

  Process {
    id: cliProcess
    running: false
  }

  Process {
    id: pollStatusProcess
    command: ["yamarchy", "status", "--json"]
    running: false
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var parsed = Model.parseState(text)
        if (parsed) root.applyState(parsed)
      }
    }
  }

  Timer {
    interval: root.opened ? 1000 : 3000
    running: true
    repeat: true
    triggeredOnStart: true
    onTriggered: {
      if (!pollStatusProcess.running) {
        pollStatusProcess.running = true
      }
    }
  }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    slotSize: Style.bar.iconSlot
    tooltipText: root.trackTitle !== "Yamarchy (Stopped)" ? (root.trackTitle + (root.trackArtist !== "" ? " — " + root.trackArtist : "")) : "Yandex Music"

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
      else root.toggle()
    }

    onWheelMoved: function(delta) {
      var steps = delta > 0 ? 5 : -5
      var newVol = Math.max(0, Math.min(100, Math.round(root.playerVolume * 100) + steps))
      root.sendCli(["volume", newVol.toString()])
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(390))
    contentHeight: panel.fittedContentHeight(panelColumn.implicitHeight)

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onCloseRequested: root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }

      Column {
        id: panelColumn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Style.space(14)

        // ==========================================
        // HERO HEADER (Player View or Settings View)
        // ==========================================
        Item {
          width: parent.width
          implicitHeight: Math.max(headerLeftGroup.implicitHeight, headerRightGroup.implicitHeight)

          Row {
            id: headerLeftGroup
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.space(10)

            // Back button when in settings view
            Rectangle {
              visible: root.currentView === "settings"
              width: 32
              height: 32
              radius: 16
              anchors.verticalCenter: parent.verticalCenter
              color: backBtnArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : "transparent"
              border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
              border.width: 1

              Text {
                anchors.centerIn: parent
                text: Model.Icons.chevronLeft
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.iconFontFamily
                font.pixelSize: 14
              }

              MouseArea {
                id: backBtnArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.currentView = "player"
              }

              PanelToolTip {
                visible: backBtnArea.containsMouse
                text: "Back to player"
                fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
              }
            }

            // Hero Icon when in player view
            YandexMusicIcon {
              visible: root.currentView === "player"
              iconSize: Style.space(26)
              color: root.bar ? root.bar.foreground : Color.accent
              anchors.verticalCenter: parent.verticalCenter
            }

            Column {
              anchors.verticalCenter: parent.verticalCenter
              spacing: 2

              Text {
                text: root.currentView === "settings" ? "Settings" : "Yandex Music"
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.title
                font.bold: true
              }

              Text {
                text: root.currentView === "settings"
                  ? "AUDIO & PLAYBACK"
                  : (root.isPlaying ? "PLAYING" : "STOPPED")
                color: (root.currentView === "player" && root.isPlaying)
                  ? (root.bar ? root.bar.foreground : Color.accent)
                  : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
                font.bold: true
                font.letterSpacing: 1.2
              }
            }
          }

          // Header Right Controls
          Row {
            id: headerRightGroup
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.space(8)

            // Settings Gear Button in Player View
            Rectangle {
              visible: root.currentView === "player"
              width: 32
              height: 32
              radius: 16
              color: settingsBtnArea.containsMouse
                ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.18)
                : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.08)
              border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.12)
              border.width: 1

              Text {
                anchors.centerIn: parent
                text: Model.Icons.gear
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.iconFontFamily
                font.pixelSize: 14
              }

              MouseArea {
                id: settingsBtnArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.currentView = "settings"
              }

              PanelToolTip {
                visible: settingsBtnArea.containsMouse
                text: "Audio & plugin settings"
                fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
              }
            }
          }
        }

        PanelSeparator {
          foreground: root.bar ? root.bar.foreground : Color.foreground
        }

        // ==========================================
        // VIEW 1: PLAYER VIEW
        // ==========================================
        Column {
          id: playerView
          width: parent.width
          spacing: Style.space(14)
          visible: root.currentView === "player"

          // Track Artwork & Info Card
          Rectangle {
            width: parent.width
            height: 74
            color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.08)
            radius: 8

            Row {
              anchors.fill: parent
              anchors.margins: Style.space(10)
              spacing: Style.space(12)

              Rectangle {
                width: 54
                height: 54
                radius: 6
                color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.12)
                clip: true

                Image {
                  anchors.fill: parent
                  source: root.artUrl
                  fillMode: Image.PreserveAspectCrop
                  visible: root.artUrl !== ""
                }

                YandexMusicIcon {
                  anchors.centerIn: parent
                  iconSize: Style.space(30)
                  color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.5)
                  visible: root.artUrl === ""
                }
              }

              Column {
                width: parent.width - 54 - Style.space(12)
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3

                Text {
                  text: root.trackTitle
                  color: root.bar ? root.bar.foreground : Color.foreground
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.body
                  font.bold: true
                  elide: Text.ElideRight
                  width: parent.width
                }

                Text {
                  text: root.trackArtist !== "" ? root.trackArtist : "Station stream"
                  color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.35)
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.caption
                  elide: Text.ElideRight
                  width: parent.width
                }
              }
            }
          }

          // Progress Bar
          Column {
            width: parent.width
            spacing: Style.space(6)

            Rectangle {
              width: parent.width
              height: Style.space(6)
              radius: 3
              color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15)

              Rectangle {
                height: parent.height
                width: root.trackLength > 0 ? (parent.width * Math.max(0, Math.min(1, root.trackPosition / root.trackLength))) : 0
                radius: 3
                color: root.bar ? root.bar.foreground : Color.accent
              }
            }

            Item {
              width: parent.width
              height: timeElapsed.implicitHeight

              Text {
                id: timeElapsed
                anchors.left: parent.left
                text: Model.formatTime(root.trackPosition * 1000)
                color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
              }

              Text {
                anchors.right: parent.right
                text: Model.formatTime(root.trackLength * 1000)
                color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
              }
            }
          }

          // Playback Buttons
          Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Style.space(16)

            // Dislike Button
            Rectangle {
              width: 40
              height: 40
              radius: 20
              color: dislikeArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : "transparent"
              border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
              border.width: 1

              Item {
                anchors.centerIn: parent
                width: 18
                height: 18

                Text {
                  anchors.centerIn: parent
                  text: Model.Icons.heartOutline
                  color: root.bar ? root.bar.foreground : Color.foreground
                  font.family: root.iconFontFamily
                  font.pixelSize: 18
                }

                Rectangle {
                  anchors.centerIn: parent
                  anchors.horizontalCenterOffset: 0.5
                  anchors.verticalCenterOffset: -0.5
                  width: 22
                  height: 1.8
                  radius: 1
                  color: root.bar ? root.bar.foreground : Color.foreground
                  rotation: -45
                  antialiasing: true
                }
              }

              MouseArea {
                id: dislikeArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.dislikeTrack()
              }
              PanelToolTip {
                visible: dislikeArea.containsMouse
                text: "Dislike & skip"
                fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
              }
            }

            // Previous Track
            Rectangle {
              width: 40
              height: 40
              radius: 20
              color: prevArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : "transparent"
              border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
              border.width: 1

              Text {
                anchors.centerIn: parent
                text: Model.Icons.prev
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.iconFontFamily
                font.pixelSize: 19
              }
              MouseArea {
                id: prevArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.prevTrack()
              }
              PanelToolTip {
                visible: prevArea.containsMouse
                text: "Previous track"
                fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
              }
            }

            // Play / Pause (Accent circle)
            Rectangle {
              width: 48
              height: 48
              radius: 24
              color: playArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.28) : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.18)

              Text {
                anchors.centerIn: parent
                text: root.isPlaying ? Model.Icons.pause : Model.Icons.play
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.iconFontFamily
                font.pixelSize: 24
              }
              MouseArea {
                id: playArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.togglePlay()
              }
              PanelToolTip {
                visible: playArea.containsMouse
                text: root.isPlaying ? "Pause" : "Play"
                fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
              }
            }

            // Next Track
            Rectangle {
              width: 40
              height: 40
              radius: 20
              color: nextArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : "transparent"
              border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
              border.width: 1

              Text {
                anchors.centerIn: parent
                text: Model.Icons.next
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.iconFontFamily
                font.pixelSize: 19
              }
              MouseArea {
                id: nextArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.nextTrack()
              }
              PanelToolTip {
                visible: nextArea.containsMouse
                text: "Next track"
                fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
              }
            }

            // Like Button
            Rectangle {
              width: 40
              height: 40
              radius: 20
              color: likeArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : "transparent"
              border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
              border.width: 1

              Text {
                anchors.centerIn: parent
                text: root.isLiked ? Model.Icons.heartFilled : Model.Icons.heartOutline
                color: root.isLiked ? (root.bar ? root.bar.urgent : Color.urgent) : (root.bar ? root.bar.foreground : Color.foreground)
                font.family: root.iconFontFamily
                font.pixelSize: 18
              }
              MouseArea {
                id: likeArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.toggleLike()
              }
              PanelToolTip {
                visible: likeArea.containsMouse
                text: root.isLiked ? "Unlike track" : "Like track"
                fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
              }
            }
          }

          PanelSeparator {
            foreground: root.bar ? root.bar.foreground : Color.foreground
          }

          // Collapsible Vibe Settings Trigger Bar
          Rectangle {
            width: parent.width
            height: 36
            radius: 7
            color: vibeTriggerArea.containsMouse
              ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.14)
              : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.07)
            border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
            border.width: 1

            Row {
              anchors.left: parent.left
              anchors.leftMargin: Style.space(10)
              anchors.verticalCenter: parent.verticalCenter
              spacing: Style.space(8)

              Text {
                text: Model.Icons.tune
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.iconFontFamily
                font.pixelSize: 14
                anchors.verticalCenter: parent.verticalCenter
              }

              Text {
                text: "CUSTOMIZE VIBE"
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
                font.bold: true
                font.letterSpacing: 1.1
                anchors.verticalCenter: parent.verticalCenter
              }
            }

            Row {
              anchors.right: parent.right
              anchors.rightMargin: Style.space(10)
              anchors.verticalCenter: parent.verticalCenter
              spacing: Style.space(6)

              Text {
                text: root.formatVibeSummary()
                color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
                anchors.verticalCenter: parent.verticalCenter
              }

              Text {
                text: Model.Icons.chevronDown
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.iconFontFamily
                font.pixelSize: 12
                anchors.verticalCenter: parent.verticalCenter
                rotation: root.vibeExpanded ? 180 : 0
                Behavior on rotation {
                  NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
                }
              }
            }

            MouseArea {
              id: vibeTriggerArea
              anchors.fill: parent
              hoverEnabled: true
              cursorShape: Qt.PointingHandCursor
              onClicked: root.vibeExpanded = !root.vibeExpanded
            }
          }

          // Collapsible Vibe Content
          Item {
            id: vibeContainer
            width: parent.width
            clip: true
            visible: height > 0
            height: root.vibeExpanded ? vibeContentColumn.implicitHeight : 0
            opacity: root.vibeExpanded ? 1.0 : 0.0

            Behavior on height {
              NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
            }
            Behavior on opacity {
              NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
            }

            Column {
              id: vibeContentColumn
              width: parent.width
              spacing: Style.space(12)

              // Stations
              Column {
                width: parent.width
                spacing: Style.space(6)

                Text {
                  text: "STATIONS"
                  color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  font.letterSpacing: 1.1
                }

                Row {
                  width: parent.width
                  spacing: Style.space(8)

                  Repeater {
                    model: Model.StationOptions

                    Rectangle {
                      required property var modelData
                      readonly property bool isSelected: root.currentStation === modelData.id
                      width: (parent.width - Style.space(24)) / 4
                      height: 30
                      radius: 6
                      color: isSelected
                        ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.25)
                        : (stationArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.07))
                      border.color: isSelected
                        ? (root.bar ? root.bar.foreground : Color.accent)
                        : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
                      border.width: 1

                      Row {
                        anchors.centerIn: parent
                        spacing: 5
                        Text {
                          text: modelData.icon
                          color: isSelected ? (root.bar ? root.bar.foreground : Color.foreground) : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.25)
                          font.family: root.iconFontFamily
                          font.pixelSize: 13
                          anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                          text: modelData.label
                          color: isSelected ? (root.bar ? root.bar.foreground : Color.foreground) : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.25)
                          font.family: root.bar ? root.bar.fontFamily : Style.font.family
                          font.pixelSize: Style.font.caption
                          font.bold: isSelected
                          anchors.verticalCenter: parent.verticalCenter
                        }
                      }
                      MouseArea {
                        id: stationArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.playStation(modelData.id)
                      }
                    }
                  }
                }
              }

              // Diversity
              Column {
                width: parent.width
                spacing: Style.space(6)

                Text {
                  text: "DIVERSITY"
                  color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  font.letterSpacing: 1.1
                }

                Row {
                  width: parent.width
                  spacing: Style.space(8)

                  Repeater {
                    model: Model.DiversityOptions

                    Rectangle {
                      required property var modelData
                      readonly property bool isSelected: root.currentDiversity === modelData.id
                      width: (parent.width - Style.space(16)) / 3
                      height: 28
                      radius: 6
                      color: isSelected
                        ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.25)
                        : (divArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.07))
                      border.color: isSelected
                        ? (root.bar ? root.bar.foreground : Color.accent)
                        : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
                      border.width: 1

                      Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: isSelected ? (root.bar ? root.bar.foreground : Color.foreground) : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.25)
                        font.family: root.bar ? root.bar.fontFamily : Style.font.family
                        font.pixelSize: Style.font.caption
                        font.bold: isSelected
                      }
                      MouseArea {
                        id: divArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.setDiversity(modelData.id)
                      }
                    }
                  }
                }
              }

              // Mood & Energy
              Column {
                width: parent.width
                spacing: Style.space(6)

                Text {
                  text: "MOOD & ENERGY"
                  color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  font.letterSpacing: 1.1
                }

                Row {
                  width: parent.width
                  spacing: Style.space(6)

                  Repeater {
                    model: Model.MoodOptions

                    Rectangle {
                      required property var modelData
                      readonly property bool isSelected: root.currentMood === modelData.id
                      width: (parent.width - Style.space(24)) / 5
                      height: 28
                      radius: 6
                      color: isSelected
                        ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.25)
                        : (moodArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.07))
                      border.color: isSelected
                        ? (root.bar ? root.bar.foreground : Color.accent)
                        : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
                      border.width: 1

                      Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: isSelected ? (root.bar ? root.bar.foreground : Color.foreground) : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.25)
                        font.family: root.bar ? root.bar.fontFamily : Style.font.family
                        font.pixelSize: Style.font.caption
                        font.bold: isSelected
                      }
                      MouseArea {
                        id: moodArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.setMood(modelData.id)
                      }
                    }
                  }
                }
              }

              // Language
              Column {
                width: parent.width
                spacing: Style.space(6)

                Text {
                  text: "LANGUAGE"
                  color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.caption
                  font.bold: true
                  font.letterSpacing: 1.1
                }

                Row {
                  width: parent.width
                  spacing: Style.space(8)

                  Repeater {
                    model: Model.LanguageOptions

                    Rectangle {
                      required property var modelData
                      readonly property bool isSelected: root.currentLanguage === modelData.id
                      width: (parent.width - Style.space(24)) / 4
                      height: 28
                      radius: 6
                      color: isSelected
                        ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.25)
                        : (langArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.07))
                      border.color: isSelected
                        ? (root.bar ? root.bar.foreground : Color.accent)
                        : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
                      border.width: 1

                      Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: isSelected ? (root.bar ? root.bar.foreground : Color.foreground) : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.25)
                        font.family: root.bar ? root.bar.fontFamily : Style.font.family
                        font.pixelSize: Style.font.caption
                        font.bold: isSelected
                      }
                      MouseArea {
                        id: langArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.setLanguage(modelData.id)
                      }
                    }
                  }
                }
              }
            }
          }
        }

        // ==========================================
        // VIEW 2: SETTINGS VIEW
        // ==========================================
        Column {
          id: settingsView
          width: parent.width
          spacing: Style.space(14)
          visible: root.currentView === "settings"

          // Settings Section Title
          Text {
            text: "AUDIO & TRANSITIONS"
            color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
            font.family: root.bar ? root.bar.fontFamily : Style.font.family
            font.pixelSize: Style.font.caption
            font.bold: true
            font.letterSpacing: 1.1
          }

          // Main Crossfade Card
          Rectangle {
            width: parent.width
            implicitHeight: xfadeCardContent.implicitHeight + Style.space(24)
            color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.08)
            radius: 8
            border.color: root.crossfadeEnabled
              ? Util.alpha(root.bar ? root.bar.foreground : Color.accent, 0.35)
              : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.08)
            border.width: 1

            Column {
              id: xfadeCardContent
              anchors.left: parent.left
              anchors.right: parent.right
              anchors.top: parent.top
              anchors.margins: Style.space(12)
              spacing: Style.space(12)

              // Crossfade Master Switch Row
              Item {
                width: parent.width
                height: 40

                Row {
                  anchors.left: parent.left
                  anchors.verticalCenter: parent.verticalCenter
                  spacing: Style.space(10)

                  Text {
                    text: Model.Icons.sparkles
                    color: root.crossfadeEnabled
                      ? (root.bar ? root.bar.foreground : Color.accent)
                      : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                    font.family: root.iconFontFamily
                    font.pixelSize: 18
                    anchors.verticalCenter: parent.verticalCenter
                  }

                  Column {
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 2

                    Text {
                      text: "Crossfading"
                      color: root.bar ? root.bar.foreground : Color.foreground
                      font.family: root.bar ? root.bar.fontFamily : Style.font.family
                      font.pixelSize: Style.font.body
                      font.bold: true
                    }

                    Text {
                      text: "Smoothly overlap consecutive songs"
                      color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                      font.family: root.bar ? root.bar.fontFamily : Style.font.family
                      font.pixelSize: Style.font.caption
                    }
                  }
                }

                // Toggle Switch Pill
                Rectangle {
                  id: xfadePill
                  anchors.right: parent.right
                  anchors.verticalCenter: parent.verticalCenter
                  width: 44
                  height: 24
                  radius: 12
                  color: root.crossfadeEnabled
                    ? (root.bar ? root.bar.foreground : Color.accent)
                    : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.18)

                  Behavior on color {
                    ColorAnimation { duration: 160 }
                  }

                  // Toggle Thumb
                  Rectangle {
                    width: 18
                    height: 18
                    radius: 9
                    color: root.bar ? root.bar.background : Color.background
                    anchors.verticalCenter: parent.verticalCenter
                    x: root.crossfadeEnabled ? parent.width - width - 3 : 3

                    Behavior on x {
                      NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
                    }
                  }
                }

                MouseArea {
                  anchors.fill: parent
                  hoverEnabled: true
                  cursorShape: Qt.PointingHandCursor
                  onClicked: root.toggleCrossfade()
                }
              }

              // Sub-settings when crossfade is enabled
              Column {
                width: parent.width
                spacing: Style.space(12)
                visible: root.crossfadeEnabled

                PanelSeparator {
                  foreground: root.bar ? root.bar.foreground : Color.foreground
                  opacity: 0.5
                }

                // Duration Selector
                Column {
                  width: parent.width
                  spacing: Style.space(6)

                  Row {
                    width: parent.width

                    Text {
                      text: "CROSSFADE DURATION"
                      color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.3)
                      font.family: root.bar ? root.bar.fontFamily : Style.font.family
                      font.pixelSize: Style.font.caption
                      font.bold: true
                      font.letterSpacing: 1.0
                    }

                    Text {
                      anchors.right: parent.right
                      text: root.crossfadeDuration + " seconds"
                      color: root.bar ? root.bar.foreground : Color.accent
                      font.family: root.bar ? root.bar.fontFamily : Style.font.family
                      font.pixelSize: Style.font.caption
                      font.bold: true
                    }
                  }

                  Row {
                    width: parent.width
                    spacing: Style.space(6)

                    Repeater {
                      model: Model.CrossfadeDurationOptions

                      Rectangle {
                        required property var modelData
                        readonly property bool isSelected: root.crossfadeDuration === modelData.id
                        width: (parent.width - Style.space(24)) / 5
                        height: 30
                        radius: 6
                        color: isSelected
                          ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.28)
                          : (durArea.containsMouse ? Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.15) : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.08))
                        border.color: isSelected
                          ? (root.bar ? root.bar.foreground : Color.accent)
                          : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.1)
                        border.width: 1

                        Text {
                          anchors.centerIn: parent
                          text: modelData.label
                          color: isSelected ? (root.bar ? root.bar.foreground : Color.foreground) : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.25)
                          font.family: root.bar ? root.bar.fontFamily : Style.font.family
                          font.pixelSize: Style.font.caption
                          font.bold: isSelected
                        }

                        MouseArea {
                          id: durArea
                          anchors.fill: parent
                          hoverEnabled: true
                          cursorShape: Qt.PointingHandCursor
                          onClicked: root.setCrossfadeDuration(modelData.id)
                        }
                      }
                    }
                  }
                }

                PanelSeparator {
                  foreground: root.bar ? root.bar.foreground : Color.foreground
                  opacity: 0.5
                }

                // Crossfade on Next Skip
                Item {
                  width: parent.width
                  height: 36

                  Row {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Style.space(10)

                    Text {
                      text: Model.Icons.next
                      color: root.crossfadeOnSkip
                        ? (root.bar ? root.bar.foreground : Color.accent)
                        : Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                      font.family: root.iconFontFamily
                      font.pixelSize: 16
                      anchors.verticalCenter: parent.verticalCenter
                    }

                    Column {
                      anchors.verticalCenter: parent.verticalCenter
                      spacing: 2

                      Text {
                        text: "Crossfade on Next button"
                        color: root.bar ? root.bar.foreground : Color.foreground
                        font.family: root.bar ? root.bar.fontFamily : Style.font.family
                        font.pixelSize: Style.font.body
                        font.bold: true
                      }

                      Text {
                        text: "Also crossfade when clicking Next"
                        color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                        font.family: root.bar ? root.bar.fontFamily : Style.font.family
                        font.pixelSize: Style.font.caption
                      }
                    }
                  }

                  Rectangle {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    width: 44
                    height: 24
                    radius: 12
                    color: root.crossfadeOnSkip
                      ? (root.bar ? root.bar.foreground : Color.accent)
                      : Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.18)

                    Behavior on color {
                      ColorAnimation { duration: 160 }
                    }

                    Rectangle {
                      width: 18
                      height: 18
                      radius: 9
                      color: root.bar ? root.bar.background : Color.background
                      anchors.verticalCenter: parent.verticalCenter
                      x: root.crossfadeOnSkip ? parent.width - width - 3 : 3

                      Behavior on x {
                        NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
                      }
                    }
                  }

                  MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.toggleCrossfadeOnSkip()
                  }
                }
              }
            }
          }

          // Audio Engine Info Box
          Rectangle {
            width: parent.width
            implicitHeight: infoBoxCol.implicitHeight + Style.space(18)
            color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.05)
            radius: 8
            border.color: Util.alpha(root.bar ? root.bar.foreground : Color.foreground, 0.08)
            border.width: 1

            Column {
              id: infoBoxCol
              anchors.left: parent.left
              anchors.right: parent.right
              anchors.top: parent.top
              anchors.margins: Style.space(10)
              spacing: Style.space(4)

              Text {
                text: "Yamarchy Native Audio Engine"
                color: root.bar ? root.bar.foreground : Color.foreground
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
                font.bold: true
              }

              Text {
                text: "Quality: High (MP3 320kbps) • Output: PipeWire / ALSA"
                color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
              }
            }
          }
        }
      }
    }
  }
}
