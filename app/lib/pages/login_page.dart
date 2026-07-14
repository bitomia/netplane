import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import '../services/netplane_service.dart';
import '../src/rust/api/client.dart';
import 'home_page.dart';

/// Connection transport options.
enum Transport { websocket, udp }

const _transportLabels = {
  Transport.websocket: 'Websocket',
  Transport.udp: 'UDP',
};

const _logoMockupSVG = '''
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" fill="none">
  <rect x="4" y="4" width="56" height="56" rx="14" fill="currentColor"/>
  <path d="M20 44V20l24 24V20" stroke="white" stroke-width="6"
        stroke-linecap="round" stroke-linejoin="round"/>
</svg>
''';

class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final _serverController = TextEditingController();
  final _linkCodeController = TextEditingController();
  Transport _transport = Transport.websocket;

  bool _connecting = false;
  String? _error;
  StreamSubscription<ConnectionEvent>? _sub;

  @override
  void dispose() {
    _sub?.cancel();
    _serverController.dispose();
    _linkCodeController.dispose();
    super.dispose();
  }

  Future<void> _connect() async {
    final host = _serverController.text.trim();
    final linkCode = _linkCodeController.text.trim();
    if (host.isEmpty || linkCode.isEmpty) {
      setState(() => _error = 'Server name and link code are required.');
      return;
    }

    setState(() {
      _connecting = true;
      _error = null;
    });

    try {
      final events = await NetplaneService.instance.startConnection(
        host: host,
        linkCode: linkCode,
        transport: _transport.name, // "websocket" | "udp"
      );

      await _sub?.cancel();
      _sub = events.listen(
        _onEvent,
        onError: (Object e) => _fail(e.toString()),
      );
    } catch (e) {
      _fail(e.toString());
    }
  }

  void _onEvent(ConnectionEvent event) {
    switch (event) {
      case ConnectionEvent_Connecting():
        break;
      case ConnectionEvent_Connected():
        if (!mounted) return;
        print(event.ipAddr);
        Navigator.of(
          context,
        ).pushReplacement(MaterialPageRoute(builder: (_) => const HomePage()));
      case ConnectionEvent_Disconnected():
        if (mounted) setState(() => _connecting = false);
      case ConnectionEvent_Error(:final field0):
        _fail(field0);
    }
  }

  void _fail(String message) {
    if (kDebugMode) {
      print(message);
    }

    if (!mounted) return;
    setState(() {
      _connecting = false;
      _error = message;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = ShadTheme.of(context);

    return Scaffold(
      backgroundColor: theme.colorScheme.background,
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 360),
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SvgPicture.string(
                    _logoMockupSVG,
                    width: 72,
                    height: 72,
                    colorFilter: ColorFilter.mode(
                      theme.colorScheme.primary,
                      BlendMode.srcIn,
                    ),
                  ),
                  const SizedBox(height: 24),
                  ShadInput(
                    controller: _serverController,
                    placeholder: const Text('Enter server name'),
                  ),
                  const SizedBox(height: 12),
                  ShadInput(
                    controller: _linkCodeController,
                    placeholder: const Text('Enter link code'),
                  ),
                  const SizedBox(height: 12),
                  ShadSelect<Transport>(
                    initialValue: _transport,
                    placeholder: const Text('Select transport'),
                    options: Transport.values
                        .map(
                          (t) => ShadOption(
                            value: t,
                            child: Text(_transportLabels[t]!),
                          ),
                        )
                        .toList(),
                    selectedOptionBuilder: (context, value) =>
                        Text(_transportLabels[value]!),
                    onChanged: (value) {
                      if (value != null) setState(() => _transport = value);
                    },
                  ),
                  if (_error != null) ...[
                    const SizedBox(height: 12),
                    Text(
                      _error!,
                      style: theme.textTheme.small.copyWith(
                        color: theme.colorScheme.destructive,
                      ),
                      textAlign: TextAlign.center,
                    ),
                  ],
                  const SizedBox(height: 24),
                  ShadButton(
                    enabled: !_connecting,
                    onPressed: _connect,
                    child: Text(_connecting ? 'Connecting…' : 'Connect'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
