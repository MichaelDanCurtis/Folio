// Several open files (D01.07) and the restored session (D01.09). Each tab is
// a workspace with its own engine session and recovery file; only the
// active tab takes keyboard focus, animates and owns the native menus.
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' show AppExitResponse;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'core.dart';

const lifecycleChannel = MethodChannel('folio/lifecycle');

/// One open workspace. The first tab (`main`) uses the engine's default
/// session and the data folder's own recovery file.
class WorkspaceTab {
  final String id;
  final WorkspaceModel model;
  final GlobalKey key = GlobalKey();
  WorkspaceTab(this.id, this.model);

  String get title {
    final title = '${model.workspace['title'] ?? ''}'.trim();
    return title.isEmpty ? 'Untitled' : title;
  }
}

/// Where a tab keeps its recovery file.
String workspaceTabRecoveryPath(String dataDirectory, String id) => id == 'main'
    ? '$dataDirectory/workspace.folio'
    : '$dataDirectory/tabs/$id/workspace.folio';

/// Opens the workspace a tab keeps in [dataDirectory].
Future<WorkspaceModel> openWorkspaceTab(String dataDirectory, String id) async {
  final path = workspaceTabRecoveryPath(dataDirectory, id);
  await Directory(File(path).parent.path).create(recursive: true);
  final fresh = !File(path).existsSync();
  final model = WorkspaceModel(
    CoreBridge(session: id == 'main' ? null : id),
    path,
  );
  await model.initialize();
  // A new tab starts blank, not with the sample workspace.
  if (fresh) await model.newWorkspace();
  return model;
}

/// The tabs a data folder had open, and which was in front.
({List<String> ids, int active}) readOpenTabs(String dataDirectory) {
  try {
    final file = File('$dataDirectory/tabs.json');
    if (!file.existsSync()) return (ids: const ['main'], active: 0);
    final stored = jsonDecode(file.readAsStringSync()) as Map;
    final ids = [
      for (final id in stored['tabs'] as List? ?? const [])
        if (id is String &&
            RegExp(r'^[a-z0-9]{1,24}$').hasMatch(id) &&
            (id == 'main' ||
                File(workspaceTabRecoveryPath(dataDirectory, id)).existsSync()))
          id,
    ];
    if (ids.isEmpty) return (ids: const ['main'], active: 0);
    final active = (stored['active'] as num?)?.toInt() ?? 0;
    return (ids: ids, active: active.clamp(0, ids.length - 1));
  } catch (_) {
    return (ids: const ['main'], active: 0);
  }
}

class WorkspaceTabs extends StatefulWidget {
  /// The workspace the app opened with (the `main` tab).
  final WorkspaceModel first;
  final String dataDirectory;
  final Widget Function(
    BuildContext context,
    WorkspaceTab tab,
    bool active,
    WorkspaceTabsState host,
  )
  buildTab;

  /// Asks a tab's workspace to save or discard its work before it closes.
  final Future<bool> Function(WorkspaceTab tab) prepareClose;

  /// Runs Open in a tab.
  final Future<void> Function(WorkspaceTab tab) open;

  /// Follows the tab in front, for app-wide settings that come from it.
  final ValueNotifier<WorkspaceModel>? activeModel;

  /// Reopens the tabs the data folder had open when it was last closed.
  final bool restore;

  const WorkspaceTabs({
    super.key,
    required this.first,
    required this.dataDirectory,
    required this.buildTab,
    required this.prepareClose,
    required this.open,
    this.activeModel,
    this.restore = true,
  });

  @override
  State<WorkspaceTabs> createState() => WorkspaceTabsState();
}

class WorkspaceTabsState extends State<WorkspaceTabs> {
  final tabs = <WorkspaceTab>[];
  int active = 0;
  bool _busy = false;
  AppLifecycleListener? _lifecycle;

  WorkspaceTab get current => tabs[active];

  @override
  void initState() {
    super.initState();
    tabs.add(WorkspaceTab('main', widget.first));
    lifecycleChannel.setMethodCallHandler((call) async {
      if (call.method == 'prepareClose') return prepareCloseAll();
      return null;
    });
    _lifecycle = AppLifecycleListener(
      onExitRequested: () async => await prepareCloseAll()
          ? AppExitResponse.exit
          : AppExitResponse.cancel,
    );
    if (widget.restore) unawaited(_restore());
  }

  Future<void> _restore() async {
    final stored = readOpenTabs(widget.dataDirectory);
    if (stored.ids.length == 1 && stored.ids.single == 'main') return;
    final restored = <WorkspaceTab>[];
    for (final id in stored.ids) {
      if (id == 'main') {
        restored.add(tabs.first);
        continue;
      }
      try {
        restored.add(
          WorkspaceTab(id, await openWorkspaceTab(widget.dataDirectory, id)),
        );
      } catch (_) {
        // A tab whose recovery file cannot be read stays closed; its file is
        // kept for Recent Files and Recovery.
      }
    }
    if (!mounted) {
      for (final tab in restored) {
        if (tab.id != 'main') await _release(tab);
      }
      return;
    }
    // A closed first tab keeps no work (its recovery was emptied on close).
    if (!stored.ids.contains('main') &&
        restored.isNotEmpty &&
        widget.first.snapshot['blankWorkspace'] == true) {
      widget.first.dispose();
      setState(() {
        tabs
          ..clear()
          ..addAll(restored);
        active = stored.active.clamp(0, tabs.length - 1);
      });
    } else {
      setState(() {
        final wanted = stored.ids.contains('main')
            ? restored
            : [tabs.first, ...restored];
        tabs
          ..clear()
          ..addAll(wanted);
        active = stored.ids.contains('main')
            ? stored.active.clamp(0, tabs.length - 1)
            : 0;
      });
    }
    _publishActive();
  }

  void _publishActive() {
    widget.activeModel?.value = current.model;
  }

  /// Records the open tabs so the next launch restores them.
  void _remember() {
    final file = File('${widget.dataDirectory}/tabs.json');
    try {
      if (tabs.length == 1 && tabs.single.id == 'main') {
        if (file.existsSync()) file.deleteSync();
        return;
      }
      file.writeAsStringSync(
        jsonEncode({
          'tabs': [for (final tab in tabs) tab.id],
          'active': active,
        }),
      );
    } catch (_) {
      // The tab list is a convenience; the workspaces have their own
      // recovery files.
    }
  }

  String _newId() {
    if (!tabs.any((t) => t.id == 'main')) return 'main';
    final random = DateTime.now().microsecondsSinceEpoch.toRadixString(36);
    var id = 't$random';
    while (tabs.any((t) => t.id == id) ||
        Directory('${widget.dataDirectory}/tabs/$id').existsSync()) {
      id = '${id}x';
    }
    return id;
  }

  void select(int index) {
    if (index < 0 || index >= tabs.length || index == active) return;
    setState(() => active = index);
    _publishActive();
    _remember();
  }

  /// Opens a blank workspace in a new tab and brings it to the front.
  Future<WorkspaceTab?> newTab() async {
    if (_busy) return null;
    _busy = true;
    try {
      final id = _newId();
      final model = await openWorkspaceTab(widget.dataDirectory, id);
      if (!mounted) {
        model.dispose();
        await model.core.close();
        return null;
      }
      final tab = WorkspaceTab(id, model);
      setState(() {
        tabs.add(tab);
        active = tabs.length - 1;
      });
      _publishActive();
      _remember();
      return tab;
    } finally {
      _busy = false;
    }
  }

  /// Runs Open in a new tab. A tab left blank by a cancelled Open closes.
  Future<void> openInNewTab() async {
    final tab = await newTab();
    if (tab == null) return;
    await WidgetsBinding.instance.endOfFrame;
    await widget.open(tab);
    if (!mounted || !tabs.contains(tab)) return;
    if (tab.model.snapshot['blankWorkspace'] == true) {
      await _remove(tab);
    }
  }

  /// Closes a tab after its workspace saves or discards its work. The last
  /// tab stays open.
  Future<bool> closeTab(WorkspaceTab tab) async {
    if (tabs.length < 2 || _busy || !tabs.contains(tab)) return false;
    _busy = true;
    try {
      final index = tabs.indexOf(tab);
      if (index != active) {
        select(index);
        await WidgetsBinding.instance.endOfFrame;
      }
      if (!await widget.prepareClose(tab)) return false;
      if (!mounted) return false;
      await tab.model.discardRecoveryOnClose();
      await _remove(tab);
      return true;
    } finally {
      _busy = false;
    }
  }

  Future<void> _remove(WorkspaceTab tab) async {
    final index = tabs.indexOf(tab);
    if (index < 0) return;
    setState(() {
      tabs.removeAt(index);
      if (active > index || active >= tabs.length) active--;
      if (active < 0) active = 0;
    });
    _publishActive();
    _remember();
    // The workspace screen lets go of the model when it leaves the tree.
    await WidgetsBinding.instance.endOfFrame;
    await _release(tab);
  }

  Future<void> _release(WorkspaceTab tab) async {
    tab.model.dispose();
    if (tab.id == 'main') return;
    try {
      await tab.model.core.close();
      await Directory(
        '${widget.dataDirectory}/tabs/${tab.id}',
      ).delete(recursive: true);
    } catch (_) {
      // Left for the next launch's clean-up; the tab is no longer listed.
    }
  }

  /// Prepares every tab for the app to quit, front tab first. Each tab that
  /// has unsaved work comes to the front to ask.
  Future<bool> prepareCloseAll() async {
    final order = [current, ...tabs.where((t) => t != current)];
    for (final tab in order) {
      final index = tabs.indexOf(tab);
      if (index < 0) continue;
      if (index != active) {
        select(index);
        await WidgetsBinding.instance.endOfFrame;
      }
      if (!await widget.prepareClose(tab)) return false;
    }
    _remember();
    return true;
  }

  @override
  void dispose() {
    lifecycleChannel.setMethodCallHandler(null);
    _lifecycle?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (tabs.length > 1) _strip(),
        Expanded(
          child: Stack(
            fit: StackFit.expand,
            children: [
              for (final (index, tab) in tabs.indexed)
                KeyedSubtree(
                  key: ValueKey('workspace-tab-${tab.id}'),
                  child: Offstage(
                    offstage: index != active,
                    child: TickerMode(
                      enabled: index == active,
                      child: ExcludeFocus(
                        excluding: index != active,
                        child: widget.buildTab(
                          context,
                          tab,
                          index == active,
                          this,
                        ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _strip() => Material(
    key: const ValueKey('workspace-tab-strip'),
    color: const Color(0xffebe9e3),
    child: SizedBox(
      height: 32,
      child: Row(
        children: [
          Expanded(
            child: ListView(
              scrollDirection: Axis.horizontal,
              children: [
                for (final (index, tab) in tabs.indexed)
                  AnimatedBuilder(
                    animation: tab.model,
                    builder: (context, _) => _TabChip(
                      key: ValueKey('workspace-tab-chip-${tab.id}'),
                      title: tab.title,
                      selected: index == active,
                      onSelect: () => select(index),
                      onClose: () => unawaited(closeTab(tab)),
                    ),
                  ),
              ],
            ),
          ),
          IconButton(
            key: const ValueKey('workspace-tab-new'),
            tooltip: 'New tab',
            iconSize: 16,
            onPressed: () => unawaited(newTab()),
            icon: const Icon(Icons.add),
          ),
        ],
      ),
    ),
  );
}

class _TabChip extends StatelessWidget {
  final String title;
  final bool selected;
  final VoidCallback onSelect, onClose;
  const _TabChip({
    super.key,
    required this.title,
    required this.selected,
    required this.onSelect,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) => Semantics(
    selected: selected,
    button: true,
    label: title,
    child: InkWell(
      onTap: onSelect,
      child: Container(
        constraints: const BoxConstraints(minWidth: 120, maxWidth: 220),
        padding: const EdgeInsets.only(left: 12, right: 2),
        decoration: BoxDecoration(
          color: selected ? const Color(0xfff8f7f3) : null,
          border: const Border(right: BorderSide(color: Color(0xffd6d3cb))),
        ),
        child: Row(
          children: [
            Expanded(
              child: Text(
                title,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                ),
              ),
            ),
            IconButton(
              tooltip: 'Close tab',
              iconSize: 13,
              onPressed: onClose,
              icon: const Icon(Icons.close),
            ),
          ],
        ),
      ),
    ),
  );
}
