import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

Panel {
  id: root
  moduleName: "jltrench.walls"
  manageIpc: false

  property var anchorItem: null
  property var hostWidget: null

  // The binary ships inside the plugin folder (see README / make install).
  readonly property string binPath: Qt.resolvedUrl("bin/walls").toString().replace("file://", "")

  property string mode: "latest" // latest | toplist | random | saved
  readonly property var modes: [
    { id: "latest", label: "Latest" },
    { id: "toplist", label: "Top" },
    { id: "random", label: "Random" },
    { id: "saved", label: "Saved" }
  ]
  readonly property var topRanges: ["1d", "3d", "1w", "1M", "3M", "6M", "1y"]
  property int rangeIndex: 3

  property var results: []
  property var chips: []
  property var saved: []
  property string activeColor: "" // wallhaven palette hex from a theme chip
  property bool searching: false // query/color search overrides the tab listing
  property int page: 1
  readonly property int pageSize: 24
  property string seed: "" // random-mode pagination continuity
  property bool busy: false
  property bool hasQuery: false
  property string statusText: ""

  function open(payload) {
    if (payload) {
      try {
        var args = JSON.parse(payload) || {}
        if (args.mode && root.mode !== args.mode) root.mode = args.mode
      } catch (e) {}
    }
    root.controller.show()
    if (root.mode === "saved") {
      loadSaved()
    } else if (!root.hasQuery && root.results.length === 0 && !root.busy) {
      root.runSearch(1)
    }
    Qt.callLater(function() {
      if (root.searching) searchField.forceActiveFocus()
    })
  }

  function close() {
    root.controller.hide()
  }

  function switchPanel(direction) {
    if (root.bar && typeof root.bar.switchPanelFrom === "function")
      return root.bar.switchPanelFrom(root.hostWidget || root, direction)
    return false
  }

  function setMode(next) {
    if (root.busy || root.mode === next) return
    root.mode = next
    root.searching = false
    root.page = 1
    root.seed = ""
    if (next === "saved") {
      loadSaved()
    } else {
      root.runSearch(1)
    }
  }

  // Enter/Go: run a query (and/or theme color) search from any tab.
  function startSearch() {
    if (root.busy) return
    if (searchField.text.trim() === "" && root.activeColor === "") {
      root.statusText = "Type a query or pick a theme color"
      return
    }
    root.searching = true
    root.page = 1
    root.seed = ""
    root.runSearch(1)
  }

  // Clear the query and restore the current tab's listing.
  function clearSearch() {
    searchField.text = ""
    root.activeColor = ""
    root.searching = false
    root.page = 1
    root.seed = ""
    root.runSearch(1)
  }

  function loadSaved() {
    if (root.busy) return
    listProc.command = [root.binPath, "list"]
    listProc.running = true
  }

  function savedAction(action, path) {
    if (root.busy || !path) return
    savedProc.action = action
    savedProc.target = path
    savedProc.errText = ""
    var name = path.split("/").pop()
    root.statusText = (action === "set" ? "Applying " : "Removing ") + name + "..."
    savedProc.command = [root.binPath, action, path]
    savedProc.running = true
  }

  function buildArgs(target) {
    var args = []
    var q = searchField.text.trim()
    if (root.searching) {
      if (q) args.push(q)
      if (root.activeColor !== "") args.push("--color", root.activeColor)
    } else if (root.mode === "latest") {
      args.push("--sort", "date_added")
    } else if (root.mode === "toplist") {
      args.push("--sort", "toplist", "--range", root.topRanges[root.rangeIndex])
    } else if (root.mode === "random") {
      args.push("--sort", "random")
      if (target > 1 && root.seed !== "") args.push("--seed", root.seed)
    }
    args.push("--page", String(target))
    return args
  }

  function runSearch(target) {
    if (root.busy) return
    target = Math.max(1, target || 1)
    if (root.searching && searchField.text.trim() === "" && root.activeColor === "") {
      root.statusText = "Type a query or pick a theme color"
      return
    }

    var args = [root.binPath, "search"].concat(root.buildArgs(target))
    searchProc.command = args
    searchProc.running = true
  }

  function apply(index) {
    if (root.busy) return
    var wp = root.results[index]
    if (!wp || !wp.id) return
    applyProc.errText = ""
    root.statusText = "Downloading " + wp.id + " (" + wp.resolution + ")..."
    applyProc.command = [root.binPath, "apply", wp.id]
    applyProc.running = true
  }

  function goNextPage() {
    if (!root.busy && root.results.length >= root.pageSize)
      runSearch(root.page + 1)
  }

  function goPrevPage() {
    if (!root.busy && root.page > 1)
      runSearch(root.page - 1)
  }

  function loadChips() {
    chipsProc.command = [root.binPath, "theme-colors"]
    chipsProc.running = true
  }

  onOpenedChanged: {
    if (root.opened) {
      loadChips()
      if (root.searching)
        Qt.callLater(function() { searchField.forceActiveFocus() })
    }
  }

  Process {
    id: chipsProc

    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        try {
          var parsed = JSON.parse(String(text))
          root.chips = parsed.colors || []
        } catch (e) {
          root.chips = []
        }
      }
    }
  }

  Process {
    id: searchProc

    property int pendingPage: 1

    onStarted: {
      root.busy = true
      pendingPage = parseInt(command[command.length - 1]) || 1
    }

    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        root.busy = false
        var raw = String(text || "").trim()

        if (raw === "") {
          // Spawn failure or crash before any output; stderr is in the log.
          root.results = []
          root.statusText = searchProc.exitedCode !== 0
            ? "walls exited with code " + searchProc.exitedCode
            : "walls produced no output"
          console.error("walls debug - empty stdout, exit:", searchProc.exitedCode,
                        "| cmd:", JSON.stringify(searchProc.command))
          return
        }

        var parsed
        try {
          parsed = JSON.parse(raw)
        } catch (e) {
          console.error("walls debug - parse exception:", e,
                        "| len:", raw.length, "| head:", raw.slice(0, 120))
          root.results = []
          root.statusText = "Unexpected walls output"
          return
        }

        if (parsed.error) {
          root.results = []
          root.statusText = parsed.error
          return
        }

        root.results = parsed.results || []
        root.page = parsed.page || searchProc.pendingPage
        root.seed = parsed.seed || ""
        root.hasQuery = true
        root.statusText = root.results.length + " wallpapers - page " + root.page +
          (root.mode === "random" ? " (seed " + root.seed + ")" : "")
      }
    }

    stderr: StdioCollector {
      waitForEnd: true
      onStreamFinished: console.warn("walls:", String(text || "").trim())
    }

    onExited: function(exitCode) {
      root.busy = false
    }
  }

  Process {
    id: applyProc

    property string errText: ""

    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: console.log("walls applied:", String(text || "").trim())
    }

    stderr: StdioCollector {
      waitForEnd: true
      onStreamFinished: applyProc.errText = String(text || "").trim()
    }

    onExited: function(exitCode) {
      root.busy = false
      if (exitCode === 0) {
        root.statusText = "Wallpaper applied to theme!"
      } else {
        var t = applyProc.errText.replace(/^walls:\s*/, "")
        root.statusText = t !== "" ? t : "Failed to apply wallpaper"
      }
    }
  }

  Process {
    id: listProc

    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        try {
          var parsed = JSON.parse(String(text))
          root.saved = parsed.wallpapers || []
          var currentName = parsed.current ? parsed.current.split("/").pop() : "none"
          root.statusText = root.saved.length + " wallpapers in \"" + (parsed.theme || "?") +
            "\" - current: " + currentName
        } catch (e) {
          console.error("walls debug - list parse exception:", e, "| head:", String(text).slice(0, 120))
          root.saved = []
          root.statusText = "Failed to read saved wallpapers"
        }
      }
    }

    stderr: StdioCollector {
      waitForEnd: true
      onStreamFinished: console.warn("walls:", String(text || "").trim())
    }
  }

  Process {
    id: savedProc

    property string action: "" // "set" | "remove"
    property string target: ""
    property string errText: ""

    onStarted: root.busy = true

    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: console.log("walls", savedProc.action + ":", String(text || "").trim())
    }

    stderr: StdioCollector {
      waitForEnd: true
      onStreamFinished: savedProc.errText = String(text || "").trim()
    }

    onExited: function(exitCode) {
      root.busy = false
      var name = savedProc.target.split("/").pop()
      if (exitCode === 0) {
        root.statusText = (savedProc.action === "set" ? "Applied: " : "Removed: ") + name
      } else {
        var t = savedProc.errText.replace(/^walls:\s*/, "")
        root.statusText = t !== "" ? t : (savedProc.action === "set" ? "Failed to apply" : "Failed to remove")
      }
      loadSaved()
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: root.anchorItem
    owner: root.hostWidget || root
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(420))
    contentHeight: panel.fittedContentHeight(content.implicitHeight)

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onCloseRequested: root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }

      Column {
        id: content
        width: parent.width
        spacing: Style.space(10)

        // Mode tabs; the toplist range selector joins the row when relevant.
        Row {
          width: parent.width
          spacing: Style.space(6)

          Repeater {
            model: root.modes

            Button {
              required property var modelData

              text: modelData.label
              selected: root.mode === modelData.id
              enabled: !root.busy
              onClicked: root.setMode(modelData.id)
            }
          }

          Button {
            visible: root.mode === "toplist"
            text: root.topRanges[root.rangeIndex]
            enabled: !root.busy
            tooltipText: "Toplist range"
            onClicked: {
              root.rangeIndex = (root.rangeIndex + 1) % root.topRanges.length
              root.runSearch(1)
            }
          }
        }

        // Query row.
        Row {
          visible: root.mode !== "saved"
          width: parent.width
          spacing: Style.space(8)

          TextField {
            id: searchField
            width: parent.width - searchButton.width - clearButton.width - parent.spacing * 2
            placeholderText: "Search wallpapers (mountains, cyberpunk...)"
            enabled: !root.busy
            Keys.onReturnPressed: root.startSearch()
            Keys.onEnterPressed: root.startSearch()
            Keys.onEscapePressed: function(event) {
              if (text !== "")
                text = ""
              else
                root.close()
              event.accepted = true
            }
          }

          Button {
            id: clearButton
            anchors.verticalCenter: parent.verticalCenter
            text: "\uf00d"
            visible: root.searching
            tooltipText: "Clear search"
            onClicked: root.clearSearch()
          }

          Button {
            id: searchButton
            anchors.verticalCenter: parent.verticalCenter
            text: root.busy ? "..." : "Go"
            onClicked: root.startSearch()
          }
        }

        // Theme color chips (from the active omarchy theme).
        Row {
          visible: root.mode !== "saved" && root.chips.length > 0
          width: parent.width
          spacing: Style.space(8)

          Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.chips.length > 0 ? "Theme:" : ""
            color: Util.alpha(root.barForeground, 0.7)
            font.pixelSize: Style.font.bodySmall
          }

          Repeater {
            model: root.chips

            delegate: Rectangle {
              required property var modelData

              width: Style.space(20)
              height: Style.space(20)
              radius: width / 2
              color: "#" + modelData.hex
              border.width: root.activeColor === modelData.matched ? 2 : 1
              border.color: root.activeColor === modelData.matched
                ? Color.accent
                : Util.alpha(root.barForeground, 0.35)

              HoverHandler {
                cursorShape: Qt.PointingHandCursor
                onHoveredChanged: {
                  if (hovered)
                    root.statusText = modelData.name + " (#" + modelData.hex + " ~ #" + modelData.matched + ")"
                }
              }

              TapHandler {
                onTapped: {
                  if (root.activeColor === modelData.matched)
                    root.activeColor = ""
                  else
                    root.activeColor = modelData.matched
                  root.startSearch()
                }
              }
            }
          }

          Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.chips.length > 0
            text: root.activeColor !== "" ? "(on)" : ""
            color: Util.alpha(root.barForeground, 0.5)
            font.pixelSize: Style.font.bodySmall
          }
        }

        ResultGrid {
          width: parent.width
          height: Style.space(252)
          visible: root.mode !== "saved" && root.results.length > 0
          results: root.results
          onActivated: function(index) { root.apply(index) }
        }

        SavedGrid {
          width: parent.width
          height: Style.space(252)
          visible: root.mode === "saved"
          items: root.saved
          onApplyRequested: function(path) { root.savedAction("set", path) }
          onRemoveRequested: function(path) { root.savedAction("remove", path) }
        }

        Text {
          visible: root.results.length === 0 && !root.busy && root.mode !== "saved"
          width: parent.width
          horizontalAlignment: Text.AlignHCenter
          text: root.statusText
          color: Util.alpha(root.barForeground, 0.6)
          font.pixelSize: Style.font.body
          elide: Text.ElideRight
        }

        Text {
          visible: root.mode === "saved" && root.saved.length === 0 && !root.busy
          width: parent.width
          horizontalAlignment: Text.AlignHCenter
          text: "No wallpapers yet - save some from the other tabs"
          color: Util.alpha(root.barForeground, 0.6)
          font.pixelSize: Style.font.body
        }

        // Pagination + status line.
        Item {
          width: parent.width
          height: Style.space(30)

          Button {
            id: prevButton
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            visible: root.mode !== "saved"
            text: "\uf053"
            enabled: !root.busy && root.page > 1
            onClicked: root.goPrevPage()
          }

          Button {
            id: nextButton
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            visible: root.mode !== "saved"
            text: "\uf054"
            enabled: !root.busy && root.results.length >= root.pageSize
            onClicked: root.goNextPage()
          }

          Text {
            anchors.centerIn: parent
            width: parent.width - prevButton.width - nextButton.width - Style.space(16)
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
            text: root.statusText
            color: root.barForeground
            opacity: root.busy ? 0.7 : 1
            font.pixelSize: Style.font.body
          }
        }
      }
    }
  }
}
