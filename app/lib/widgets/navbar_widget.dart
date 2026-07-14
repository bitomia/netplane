import 'package:flutter/material.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// A mockup logo drawn as an inline SVG.
const _logoSvg = '''
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" fill="none">
  <rect x="4" y="4" width="56" height="56" rx="14" fill="currentColor"/>
  <path d="M20 44V20l24 24V20" stroke="white" stroke-width="6"
        stroke-linecap="round" stroke-linejoin="round"/>
</svg>
''';

/// Top navigation bar: logo on the left, an icon avatar on the right.
class NavbarWidget extends StatelessWidget {
  const NavbarWidget({
    super.key,
    this.onAvatarPressed,
  });

  /// Called when the avatar is tapped.
  final VoidCallback? onAvatarPressed;

  @override
  Widget build(BuildContext context) {
    final theme = ShadTheme.of(context);

    return Container(
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      decoration: BoxDecoration(
        color: theme.colorScheme.background,
        border: Border(
          bottom: BorderSide(color: theme.colorScheme.border),
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          SvgPicture.string(
            _logoSvg,
            width: 32,
            height: 32,
            colorFilter: ColorFilter.mode(
              theme.colorScheme.primary,
              BlendMode.srcIn,
            ),
          ),
          ShadButton.ghost(
            padding: EdgeInsets.zero,
            onPressed: onAvatarPressed,
            child: ShadAvatar(
              '',
              size: const Size(32, 32),
              placeholder: const Icon(LucideIcons.user, size: 18),
            ),
          ),
        ],
      ),
    );
  }
}
