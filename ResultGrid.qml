import QtQuick
import qs.Commons

// Thumbnail grid for search results. Clicking a tile activates it; the panel
// owns downloading/applying and status reporting. Hovering shows a compact
// info overlay (id, resolution, size, views, favorites) like the wallhaven
// wallpaper page.
GridView {
  id: grid

  property var results: []
  signal activated(int index)

  function fmtCount(n) {
    if (n >= 1000000) return (n / 1000000).toFixed(1) + "M"
    if (n >= 1000) return (n / 1000).toFixed(1) + "k"
    return String(n)
  }

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
      clip: true

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

      // Hover info overlay.
      Rectangle {
        anchors.fill: parent
        visible: hover.hovered && wp
        color: Util.alpha("#000000", 0.62)

        Column {
          anchors.fill: parent
          anchors.margins: Style.space(5)
          spacing: Style.space(2)

          Text {
            width: parent.width
            text: wp ? wp.id : ""
            color: "#ffffff"
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
            font.bold: true
            elide: Text.ElideRight
          }

          Text {
            width: parent.width
            text: wp ? wp.resolution + "  " + wp.size : ""
            color: Util.alpha("#ffffff", 0.85)
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
            elide: Text.ElideRight
          }

          Text {
            width: parent.width
            text: wp ? "\uf06e " + grid.fmtCount(wp.views) + "   \uf004 " + grid.fmtCount(wp.favorites) : ""
            color: Util.alpha("#ffffff", 0.7)
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
            elide: Text.ElideRight
          }
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

