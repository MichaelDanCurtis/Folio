import 'dart:convert';

const _toggles = [
  'bold',
  'italic',
  'strike',
  'caps',
  'smallCaps',
  'outline',
  'shadow',
  'emboss',
  'imprint',
  'hidden',
];
Map<String, dynamic> _copy(Map value) =>
    Map<String, dynamic>.from(jsonDecode(jsonEncode(value)));

/// Synchronous typing projection of the engine's supported style cascade.
/// The workspace engine remains authoritative for style edits and validation.
Map<String, dynamic> resolveNamedStyle(Map catalog, String? id) {
  final styles = {
    for (final s in catalog['styles'] as List? ?? []) s['id']: s as Map,
  };
  final out = <String, dynamic>{'paragraph': {}, 'tabs': [], 'run': {}};
  void merge(Map layer, {required bool toggle}) {
    for (final key in ['paragraph', 'run']) {
      final target = out[key] as Map;
      for (final e in (layer[key] as Map? ?? {}).entries) {
        if (key == 'run' && toggle && _toggles.contains(e.key)) {
          if (e.value == true) target[e.key] = target[e.key] != true;
        } else if (key == 'run' && e.key == 'rFonts') {
          final fonts = Map<String, dynamic>.from(target[e.key] as Map? ?? {});
          for (final pair in const [
            ('ascii', 'asciiTheme'),
            ('hAnsi', 'hAnsiTheme'),
            ('eastAsia', 'eastAsiaTheme'),
            ('cs', 'cstheme'),
          ]) {
            if ((e.value as Map).containsKey(pair.$1) ||
                (e.value as Map).containsKey(pair.$2)) {
              fonts.remove(pair.$1);
              fonts.remove(pair.$2);
            }
          }
          target[e.key] = {...fonts, ...e.value as Map};
        } else {
          target[e.key] = e.value is Map ? _copy(e.value as Map) : e.value;
        }
      }
    }
    for (final key in ['alignment', 'outlineLevel']) {
      if (layer.containsKey(key)) out[key] = layer[key];
    }
    final stops = {for (final t in out['tabs'] as List) t['position']: t};
    for (final t in layer['tabs'] as List? ?? []) {
      if (t['alignment'] == 'clear') {
        stops.remove(t['position']);
      } else {
        stops[t['position']] = _copy(t as Map);
      }
    }
    out['tabs'] = [
      for (final k
          in stops.keys.toList()
            ..sort((a, b) => (a as num).compareTo(b as num)))
        stops[k],
    ];
  }

  merge(catalog['defaults'] as Map? ?? {}, toggle: false);
  final seen = <String>{}, chain = <Map>[];
  Map? current = styles[id ?? catalog['defaultStyle']];
  while (current != null && seen.add(current['id'] as String)) {
    chain.add(current['properties'] as Map);
    final base = styles[current['basedOn']];
    current = base?['type'] == current['type'] ? base : null;
  }
  for (final layer in chain.reversed) {
    merge(layer, toggle: true);
  }
  final run = out['run'] as Map, theme = catalog['theme'] as Map? ?? {};
  final fonts = run['rFonts'] as Map?;
  if (fonts != null) {
    for (final entry in const [
      ('ascii', 'asciiTheme', 'latin'),
      ('hAnsi', 'hAnsiTheme', 'latin'),
      ('eastAsia', 'eastAsiaTheme', 'ea'),
      ('cs', 'cstheme', 'cs'),
    ]) {
      final ref = fonts[entry.$2];
      if (ref is String) {
        final face =
            theme['fonts']?[ref.startsWith('major')
                ? 'majorFont'
                : 'minorFont']?[entry.$3];
        if (face is String) fonts[entry.$1] = face;
      }
    }
  }
  final color = run['color'] as Map?;
  if (color != null &&
      !color.containsKey('themeTint') &&
      !color.containsKey('themeShade')) {
    final name = color['themeColor'],
        key =
            const {
              'background1': 'lt1',
              'text1': 'dk1',
              'background2': 'lt2',
              'text2': 'dk2',
            }[name] ??
            name;
    final value = theme['colors']?[key];
    if (value is String) color['val'] = value;
  }
  return out;
}

Map<String, dynamic> namedParagraphFormat(
  Map catalog,
  String id, {
  Map? direct,
}) {
  final p = resolveNamedStyle(catalog, id);
  return {
    'styleId': id,
    'inherited': {
      'before': 0,
      'after': 0,
      'line': 240,
      'lineRule': 'auto',
      ...p['paragraph'] as Map,
    },
    'direct': {...?direct},
    'inheritedRun': p['run'],
    if (p['alignment'] != null) 'inheritedAlignment': p['alignment'],
    if (p['outlineLevel'] != null) 'inheritedOutlineLevel': p['outlineLevel'],
  };
}
