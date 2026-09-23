/// Synchronous projection of the engine's available heading gallery. Applying
/// one materializes only that style and its dependencies in the Rust transaction.
Map? _matchingStyle(Map catalog, String id, String name) {
  final styles = (catalog['styles'] as List? ?? []).cast<Map>();
  return styles
          .where(
            (s) =>
                s['type'] == 'paragraph' &&
                (s['id'] as String).toLowerCase() == id.toLowerCase(),
          )
          .firstOrNull ??
      styles
          .where(
            (s) =>
                s['type'] == 'paragraph' &&
                (s['name'] as String).toLowerCase() == name.toLowerCase(),
          )
          .firstOrNull;
}

Map? builtinHeadingStyle(Map catalog, int level) =>
    _matchingStyle(catalog, 'Heading$level', 'Heading $level');
Map? builtinBodyStyle(Map catalog) =>
    (catalog['styles'] as List? ?? [])
        .cast<Map>()
        .where(
          (s) => s['type'] == 'paragraph' && s['id'] == catalog['defaultStyle'],
        )
        .firstOrNull ??
    _matchingStyle(catalog, 'Normal', 'Normal');

List<Map<String, dynamic>> availableBuiltinStyles(Map catalog) {
  final used = {
    for (final s in catalog['styles'] as List? ?? [])
      (s['id'] as String).toLowerCase(),
  };
  String unique(String preferred) {
    var id = preferred, suffix = 0;
    while (!used.add(id.toLowerCase())) {
      id = 'FolioBuiltin$preferred${++suffix}';
    }
    return id;
  }

  final added = <Map<String, dynamic>>[];
  final body = builtinBodyStyle(catalog);
  final bodyId = body?['id'] as String? ?? unique('Normal');
  if (body == null) {
    added.add({
      'id': bodyId,
      'name': 'Normal',
      'type': 'paragraph',
      'customStyle': false,
      'properties': {'paragraph': {}, 'tabs': [], 'run': {}},
    });
  }
  const sizes = [
    (48, 0, 270, false, false),
    (36, 270, 180, false, false),
    (29, 210, 180, false, false),
    (30, 120, 0, true, false),
    (27, 120, 0, true, false),
    (24, 120, 0, true, false),
    (24, 120, 0, true, false),
    (24, 120, 0, false, true),
    (24, 120, 0, false, true),
  ];
  for (var index = 0; index < sizes.length; index++) {
    final level = index + 1;
    if (builtinHeadingStyle(catalog, level) != null) continue;
    final (size, before, after, bold, italic) = sizes[index];
    added.add({
      'id': unique('Heading$level'),
      'name': 'Heading $level',
      'type': 'paragraph',
      'customStyle': false,
      'basedOn': bodyId,
      'next': bodyId,
      'properties': {
        'outlineLevel': index,
        'paragraph': {'before': before, 'after': after},
        'tabs': [],
        'run': {'sz': size, if (bold) 'bold': true, if (italic) 'italic': true},
      },
    });
  }
  return added;
}

Map? expandedBuiltinStyles(Map? catalog) => catalog == null
    ? null
    : {
        ...catalog,
        'styles': [
          ...catalog['styles'] as List,
          ...availableBuiltinStyles(catalog),
        ],
      };
