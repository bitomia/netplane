import 'package:flutter/material.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import '../widgets/navbar_widget.dart';
import 'settings_page.dart';

class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = ShadTheme.of(context);

    return Scaffold(
      backgroundColor: theme.colorScheme.background,
      body: SafeArea(
        child: Column(
          children: [
            NavbarWidget(
              onAvatarPressed: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const SettingsPage()),
              ),
            ),
            Expanded(
              child: Center(
                child: Text(
                  'Home',
                  style: theme.textTheme.h3,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
