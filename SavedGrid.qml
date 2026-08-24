import QtQuick
import qs.Commons
import qs.Ui

// Local wallpaper library for the active theme. Hover a tile to apply or
// remove it; removal is only offered for user-downloaded files.
GridView {
  id: grid

  property var items: []
  signal applyRequested(string path)
  signal removeRequested(string path)

  clip: true
  cellWidth: Math.floor(width / 3)
  cellHeight: Style.space(84)
  model: items.length
  boundsBehavior: Flickable.StopAtBounds

  delegate: Item {
    id: tile
    required property int index

    width: grid.cellWidth - Style.space(6)
    height: grid.cellHeight - Style.space(6)

    readonly property var wp: items[index] || null
    readonly property bool isUser: wp ? wp.source === "user" : false

    Rectangle {
      id: frame
      anchors.fill: parent
      color: Color.background
      radius: Style.cornerRadius
      border.width: wp && wp.is_current ? 2 : (hover.hovered ? 2 : 1)
      border.color: wp && wp.is_current
        ? Color.accent
        : (hover.hovered ? Color.accent : Util.alpha(Color.foreground, 0.25))
      clip: true

      Image {
        anchors.fill: parent
        anchors.margins: 1
        source: wp ? Util.fileUrl(wp.path) : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
        smooth: true
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity {
          NumberAnimation { duration: 150 }
        }
      }

      // Current-background badge.
      Rectangle {
        visible: wp && wp.is_current
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: Style.space(4)
        width: Style.space(18)
        height: Style.space(18)
        radius: width / 2
        color: Util.alpha("#000000", 0.6)

        Text {
          anchors.centerIn: parent
          text: "\uf00c"
          color: Color.accent
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
        }
      }

      // Hover actions overlay.
      Rectangle {
        anchors.fill: parent
        visible: hover.hovered && wp
        color: Util.alpha("#000000", 0.55)

        Row {
          anchors.centerIn: parent
          spacing: Style.space(8)

          Rectangle {
            width: Style.space(26)
            height: width
            radius: width / 2
            color: setArea.containsMouse ? Color.accent : Util.alpha("#000000", 0.4)
            border.width: 1
            border.color: Util.alpha("#ffffff", 0.35)

            Text {
              anchors.centerIn: parent
              text: "\uf00c"
              color: setArea.containsMouse ? "#000000" : "#ffffff"
              font.family: Style.font.family
              font.pixelSize: Style.font.body
            }

            MouseArea {
              id: setArea
              anchors.fill: parent
              hoverEnabled: true
              cursorShape: Qt.PointingHandCursor
              onClicked: grid.applyRequested(tile.wp.path)
            }
          }

          Rectangle {
            visible: tile.isUser
            width: Style.space(26)
            height: width
            radius: width / 2
            color: delArea.containsMouse ? "#cc3333" : Util.alpha("#000000", 0.4)
            border.width: 1
            border.color: Util.alpha("#ffffff", 0.35)

            Text {
              anchors.centerIn: parent
              text: "\uf1f8"
              color: "#ffffff"
              font.family: Style.font.family
              font.pixelSize: Style.font.body
            }

            MouseArea {
              id: delArea
              anchors.fill: parent
              hoverEnabled: true
              cursorShape: Qt.PointingHandCursor
              onClicked: grid.removeRequested(tile.wp.path)
            }
          }
        }
      }

      // File name caption.
      Text {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: Style.space(4)
        visible: hover.hovered
        text: wp ? wp.name : ""
        elide: Text.ElideMiddle
        horizontalAlignment: Text.AlignHCenter
        color: "#ffffff"
        style: Text.Outline
        styleColor: Util.alpha("#000000", 0.7)
        font.pixelSize: Style.font.bodySmall
      }

      HoverHandler {
        id: hover
        cursorShape: Qt.PointingHandCursor
      }
    }
  }
}
