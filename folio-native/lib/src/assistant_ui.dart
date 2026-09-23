import 'draft_text_field.dart';
import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:genui/genui.dart' as g;
import 'package:a2ui_core/a2ui_core.dart' as a;
import 'package:json_schema_builder/json_schema_builder.dart';
import 'core.dart';
import 'workbook_block.dart';

const folioCatalogId = 'com.folio.office';

/// Native A2UI host. The catalog is deliberately limited to local widgets;
/// generated UI cannot execute code, fetch media, or apply workspace edits.
class AssistantUi extends ChangeNotifier {
  final WorkspaceModel model;
  final void Function(Map<String, dynamic>) onAction;
  final void Function(String, String) onOpen;
  late final g.SurfaceController controller;
  final _definitions = <String, Map<String, Map<String, dynamic>>>{};
  final _subscriptions = <StreamSubscription>[];
  AssistantUi({
    required this.model,
    required this.onAction,
    required this.onOpen,
  }) {
    final textSchema = S.object(
      properties: {'text': S.string()},
      required: ['text'],
    );
    controller = g.SurfaceController(
      catalogs: [
        g.Catalog([
          g.CatalogItem(
            name: 'Text',
            dataSchema: textSchema,
            widgetBuilder: (c) => Padding(
              padding: const EdgeInsets.symmetric(vertical: 6),
              child: Text(
                (c.data as Map)['text'],
                style: const TextStyle(fontSize: 12, height: 1.5),
              ),
            ),
          ),
          g.CatalogItem(
            name: 'Column',
            dataSchema: S.object(
              properties: {'children': S.list(items: S.string())},
              required: ['children'],
            ),
            widgetBuilder: (c) => Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                for (final id in (c.data as Map)['children']) c.buildChild(id),
              ],
            ),
          ),
          g.CatalogItem(
            name: 'Button',
            dataSchema: S.object(
              properties: {'label': S.string(), 'action': S.string()},
              required: ['label', 'action'],
            ),
            widgetBuilder: (c) {
              final d = c.data as Map;
              return Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: OutlinedButton(
                  onPressed: () {
                    c.dispatchEvent(
                      g.UserActionEvent(
                        name: d['action'],
                        sourceComponentId: c.id,
                        context: {
                          'surfaceId': c.surfaceId,
                          'label': d['label'],
                          'fields': c.dataContext.getValue<Object>(
                            g.DataPath.root,
                          ),
                        },
                      ),
                    );
                  },
                  child: Text(d['label']),
                ),
              );
            },
          ),
          g.CatalogItem(
            name: 'TextField',
            dataSchema: S.object(
              properties: {'label': S.string(), 'path': S.string()},
              required: ['label', 'path'],
            ),
            widgetBuilder: (c) {
              final d = c.data as Map;
              return Padding(
                padding: const EdgeInsets.symmetric(vertical: 6),
                child: draftTextField(
                  formField: true,
                  key: ValueKey('${c.surfaceId}:${c.id}'),
                  initialValue:
                      c.dataContext.getValue<String>(g.DataPath(d['path'])) ??
                      '',
                  onChanged: (v) =>
                      c.dataContext.update(g.DataPath(d['path']), v),
                  decoration: InputDecoration(
                    labelText: d['label'],
                    border: const OutlineInputBorder(),
                    isDense: true,
                  ),
                ),
              );
            },
          ),
          g.CatalogItem(
            name: 'WorkbookRange',
            dataSchema: S.object(
              properties: {
                'sheetId': S.string(),
                'range': S.string(),
                'title': S.string(),
                'mode': S.string(),
              },
              required: ['sheetId', 'range'],
            ),
            widgetBuilder: (c) => WorkbookBlock(
              model: model,
              attributes: c.data as Map,
              onOpen: onOpen,
            ),
          ),
        ], catalogId: folioCatalogId),
      ],
    );
    _subscriptions.add(
      controller.surfaceUpdates.listen((_) => notifyListeners()),
    );
    _subscriptions.add(
      controller.onSubmit.listen((message) {
        for (final part in message.parts.uiInteractionParts) {
          final value = jsonDecode(part.interaction) as Map;
          onAction(Map<String, dynamic>.from(value['action'] as Map));
        }
      }),
    );
  }
  List<String> get surfaces => controller.activeSurfaceIds.toList();
  void present(List<dynamic> messages) {
    if (messages.isEmpty ||
        messages.length > 30 ||
        jsonEncode(messages).length > 60000) {
      throw const FormatException('UI payload exceeds limits');
    }
    final next = <String, Map<String, Map<String, dynamic>>>{
      for (final e in _definitions.entries) e.key: Map.from(e.value),
    };
    final parsed = <a.A2uiMessage>[];
    for (final raw in messages) {
      final m = Map<String, dynamic>.from(raw as Map);
      if (m['version'] != 'v0.9') {
        throw const FormatException('Use A2UI version v0.9');
      }
      final bodies = m.keys.where((k) => k != 'version').toList();
      if (bodies.length != 1) {
        throw const FormatException('Each message needs one operation');
      }
      final op = bodies.single;
      if (![
        'createSurface',
        'updateComponents',
        'updateDataModel',
        'deleteSurface',
      ].contains(op)) {
        throw const FormatException('Unsupported A2UI operation');
      }
      final body = Map<String, dynamic>.from(m[op] as Map);
      final id = body['surfaceId'];
      if (id is! String || !RegExp(r'^[a-zA-Z0-9_-]{1,64}$').hasMatch(id)) {
        throw const FormatException('Invalid surface id');
      }
      if (op == 'createSurface') {
        if (body['catalogId'] != folioCatalogId) {
          throw const FormatException('Use the Folio catalog');
        }
        next.putIfAbsent(id, () => {});
      }
      if (op == 'deleteSurface') {
        next.remove(id);
      }
      if (op == 'updateComponents') {
        final components = body['components'] as List;
        final map = next.putIfAbsent(id, () => {});
        for (final raw in components) {
          final c = Map<String, dynamic>.from(raw as Map);
          final cid = c['id'];
          if (cid is! String || cid.length > 64) {
            throw const FormatException('Invalid component id');
          }
          if (![
            'Text',
            'Column',
            'Button',
            'TextField',
            'WorkbookRange',
          ].contains(c['component'])) {
            throw const FormatException('Unsupported component');
          }
          if (c['component'] == 'Column' &&
              (c['children'] is! List ||
                  (c['children'] as List).any((v) => v is! String))) {
            throw const FormatException('Column needs child ids');
          }
          for (final key in switch (c['component']) {
            'Text' => ['text'],
            'Button' => ['label', 'action'],
            'TextField' => ['label', 'path'],
            'WorkbookRange' => ['sheetId', 'range'],
            _ => <String>[],
          }) {
            if (c[key] is! String || (c[key] as String).length > 6000) {
              throw FormatException('Invalid $key');
            }
          }
          if (c['component'] == 'TextField' &&
              !RegExp(r'^/[a-zA-Z][a-zA-Z0-9_]{0,40}$').hasMatch(c['path'])) {
            throw const FormatException('Use a simple absolute field path');
          }
          if (c['component'] == 'WorkbookRange') {
            rangeValues({}, c['range']);
          }
          map[cid] = c;
        }
      }
      parsed.add(a.A2uiMessage.fromJson(m));
    }
    if (next.length > 6) {
      throw const FormatException('At most six assistant forms may be open');
    }
    for (final map in next.values) {
      if (map.length > 80 || !map.containsKey('root')) {
        throw const FormatException(
          'Each surface needs root and at most 80 components',
        );
      }
      var visits = 0;
      void visit(String id, Set<String> ancestors) {
        if (++visits > 200 || ancestors.length > 12 || ancestors.contains(id)) {
          throw const FormatException('UI nesting is too deep or cyclic');
        }
        final c = map[id];
        if (c == null) throw const FormatException('Missing child component');
        if (c['component'] == 'Column') {
          for (final child in c['children']) {
            visit(child, {...ancestors, id});
          }
        }
      }

      visit('root', {});
    }
    // Validate the whole batch before allowing the renderer to see any of it.
    for (final message in parsed) {
      controller.handleMessage(message);
    }
    _definitions
      ..clear()
      ..addAll(next);
    notifyListeners();
  }

  void showChoices({
    required String id,
    required String title,
    required List<(String, String)> choices,
  }) => present([
    {
      'version': 'v0.9',
      'createSurface': {'surfaceId': id, 'catalogId': folioCatalogId},
    },
    {
      'version': 'v0.9',
      'updateComponents': {
        'surfaceId': id,
        'components': [
          {
            'id': 'root',
            'component': 'Column',
            'children': [
              'title',
              for (var i = 0; i < choices.length; i++) 'choice$i',
            ],
          },
          {'id': 'title', 'component': 'Text', 'text': title},
          for (var i = 0; i < choices.length; i++)
            {
              'id': 'choice$i',
              'component': 'Button',
              'label': choices[i].$1,
              'action': choices[i].$2,
            },
        ],
      },
    },
  ]);
  Widget surface(String id) =>
      g.Surface(surfaceContext: controller.contextFor(id));
  void close(String id) {
    controller.handleMessage(
      a.A2uiMessage.fromJson({
        'version': 'v0.9',
        'deleteSurface': {'surfaceId': id},
      }),
    );
    _definitions.remove(id);
    notifyListeners();
  }

  void clear() {
    for (final id in surfaces) {
      controller.handleMessage(
        a.A2uiMessage.fromJson({
          'version': 'v0.9',
          'deleteSurface': {'surfaceId': id},
        }),
      );
    }
    _definitions.clear();
    notifyListeners();
  }

  @override
  void dispose() {
    for (final s in _subscriptions) {
      s.cancel();
    }
    controller.dispose();
    super.dispose();
  }
}
