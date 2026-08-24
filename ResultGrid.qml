import QtQuick
import qs.Commons

// Thumbnail grid for search results. Clicking a tile activates it; the panel
// owns downloading/applying and status reporting.
GridView {
  id: grid

  property var results: []
  signal activated(int index)

  clip: true
  cellWidth: Math.floor(width / 3)
  cellHeight: Style.space(84)
  model: results.length

  boundsBehavior: Flickable.StopAtBounds

  delegate: Item {
    required property int index

    width: grid.cellWidth - Style.space(6)
    height: grid.cellHeight - Style.space(6)

    readonly property var wp: results[index] || null

    Rectangle {
      id: frame
      anchors.fill: parent
      color: Color.background
      radius: Style.cornerRadius
      border.width: hover.hovered ? 2 : 1
      border.color: hover.hovered ? Color.accent : Util.alpha(Color.foreground, 0.25)

      Image {
        anchors.fill: parent
        anchors.margins: 1
        source: wp ? wp.thumb : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
        smooth: true
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity {
          NumberAnimation { duration: 150 }
        }
      }

      HoverHandler {
        id: hover
        cursorShape: Qt.PointingHandCursor
      }

      TapHandler {
        onTapped: grid.activated(index)
      }
    }
  }
}
