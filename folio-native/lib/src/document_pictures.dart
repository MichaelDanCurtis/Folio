// Inline pictures in documents (W08.01–W08.02): the editor embed that shows
// a picture from the document's package, and inserting one from a file.
import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'core.dart';
import 'document_page_layout.dart' show previewPictures;

/// Pictures larger than this are refused, as the engine does.
const maxPictureBytes = 10 * 1024 * 1024;

/// Chooses a picture file; replaced in tests.
Future<({String name, Uint8List bytes})?> Function() choosePictureFile =
    () async {
      final file = await openFile(
        acceptedTypeGroups: const [
          XTypeGroup(
            label: 'Pictures',
            extensions: ['png', 'jpg', 'jpeg', 'gif'],
            uniformTypeIdentifiers: [
              'public.png',
              'public.jpeg',
              'com.compuserve.gif',
            ],
          ),
        ],
      );
      if (file == null) return null;
      final size = await file.length();
      if (size > maxPictureBytes) {
        throw const FormatException('Pictures are limited to 10 MB.');
      }
      return (name: file.name, bytes: await file.readAsBytes());
    };

/// A new picture's node, filled in by the engine when it is stored.
Map<String, dynamic> pendingPicture() => {
  'type': 'image',
  'attrs': {'src': 'pending', 'alt': ''},
};

/// Picture bytes by package and part, loaded once per session.
final _pictures = <String, Future<Uint8List>>{};

Future<Uint8List> pictureBytes(WorkspaceModel model, String src) {
  final package = model.workspace['officeDocument']?['packageId'] ?? '';
  return _pictures.putIfAbsent('$package|$src', () async {
    final data = await model.core.call({'op': 'documentImage', 'src': src});
    return base64Decode(data['base64'] as String);
  });
}

/// Shows an inline picture at its size, scaled down to the line's width.
class DocumentPictureBuilder extends q.EmbedBuilder {
  final WorkspaceModel? model;
  const DocumentPictureBuilder({this.model});
  @override
  String get key => 'folioImage';
  @override
  bool get expanded => false;

  @override
  Widget build(BuildContext context, q.EmbedContext embedContext) {
    final node = jsonDecode(embedContext.node.value.data) as Map;
    return DocumentPicture(node: node, model: model);
  }
}

class DocumentPicture extends StatelessWidget {
  final Map node;
  final WorkspaceModel? model;
  const DocumentPicture({required this.node, this.model, super.key});

  @override
  Widget build(BuildContext context) {
    final attrs = node['attrs'] as Map? ?? {};
    final width = (attrs['width'] as num?)?.toDouble() ?? 96;
    final height = (attrs['height'] as num?)?.toDouble() ?? 96;
    final alt = (attrs['alt'] as String?) ?? '';
    final src = attrs['src'] as String?;
    return LayoutBuilder(
      builder: (context, constraints) {
        final scale =
            constraints.maxWidth.isFinite && width > constraints.maxWidth
            ? constraints.maxWidth / width
            : 1.0;
        final size = Size(width * scale, height * scale);
        Widget placeholder([IconData icon = Icons.image_outlined]) => Container(
          color: const Color(0xfff1f3f5),
          alignment: Alignment.center,
          child: Icon(icon, size: 18, color: const Color(0xff8a939e)),
        );
        final image = model == null || src == null
            ? placeholder()
            : FutureBuilder<Uint8List>(
                future: pictureBytes(model!, src),
                builder: (context, snapshot) {
                  if (snapshot.hasError) {
                    return placeholder(Icons.broken_image_outlined);
                  }
                  if (!snapshot.hasData) return placeholder();
                  return Image.memory(
                    snapshot.data!,
                    fit: BoxFit.fill,
                    gaplessPlayback: true,
                    errorBuilder: (_, _, _) =>
                        placeholder(Icons.broken_image_outlined),
                  );
                },
              );
        return Semantics(
          key: ValueKey('document-picture-$src'),
          image: true,
          label: alt.isEmpty ? 'Picture' : alt,
          child: SizedBox.fromSize(
            size: size,
            child: ExcludeSemantics(child: image),
          ),
        );
      },
    );
  }
}

/// Decodes the pictures in a page-preview snapshot, so the layout can paint
/// them. Pictures that can't be decoded keep their placeholder.
Future<void> preloadPictures(WorkspaceModel model, Map snapshot) async {
  final sources = <String>{};
  void walk(Object? n) {
    if (n is Map) {
      if (n['type'] == 'image' && n['attrs']?['src'] is String) {
        sources.add(n['attrs']['src'] as String);
      }
      n.values.forEach(walk);
    } else if (n is List) {
      n.forEach(walk);
    }
  }

  walk(snapshot);
  previewPictures.clear();
  for (final src in sources) {
    try {
      final codec = await ui.instantiateImageCodec(
        await pictureBytes(model, src),
      );
      previewPictures[src] = (await codec.getNextFrame()).image;
    } catch (_) {
      // Shown as a placeholder.
    }
  }
}
