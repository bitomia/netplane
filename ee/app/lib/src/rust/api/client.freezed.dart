// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'client.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ConnectionEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ConnectionEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ConnectionEvent()';
}


}

/// @nodoc
class $ConnectionEventCopyWith<$Res>  {
$ConnectionEventCopyWith(ConnectionEvent _, $Res Function(ConnectionEvent) __);
}


/// Adds pattern-matching-related methods to [ConnectionEvent].
extension ConnectionEventPatterns on ConnectionEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ConnectionEvent_Connecting value)?  connecting,TResult Function( ConnectionEvent_Connected value)?  connected,TResult Function( ConnectionEvent_Disconnected value)?  disconnected,TResult Function( ConnectionEvent_Error value)?  error,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ConnectionEvent_Connecting() when connecting != null:
return connecting(_that);case ConnectionEvent_Connected() when connected != null:
return connected(_that);case ConnectionEvent_Disconnected() when disconnected != null:
return disconnected(_that);case ConnectionEvent_Error() when error != null:
return error(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ConnectionEvent_Connecting value)  connecting,required TResult Function( ConnectionEvent_Connected value)  connected,required TResult Function( ConnectionEvent_Disconnected value)  disconnected,required TResult Function( ConnectionEvent_Error value)  error,}){
final _that = this;
switch (_that) {
case ConnectionEvent_Connecting():
return connecting(_that);case ConnectionEvent_Connected():
return connected(_that);case ConnectionEvent_Disconnected():
return disconnected(_that);case ConnectionEvent_Error():
return error(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ConnectionEvent_Connecting value)?  connecting,TResult? Function( ConnectionEvent_Connected value)?  connected,TResult? Function( ConnectionEvent_Disconnected value)?  disconnected,TResult? Function( ConnectionEvent_Error value)?  error,}){
final _that = this;
switch (_that) {
case ConnectionEvent_Connecting() when connecting != null:
return connecting(_that);case ConnectionEvent_Connected() when connected != null:
return connected(_that);case ConnectionEvent_Disconnected() when disconnected != null:
return disconnected(_that);case ConnectionEvent_Error() when error != null:
return error(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  connecting,TResult Function( String ipAddr,  String netmask)?  connected,TResult Function()?  disconnected,TResult Function( String field0)?  error,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ConnectionEvent_Connecting() when connecting != null:
return connecting();case ConnectionEvent_Connected() when connected != null:
return connected(_that.ipAddr,_that.netmask);case ConnectionEvent_Disconnected() when disconnected != null:
return disconnected();case ConnectionEvent_Error() when error != null:
return error(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  connecting,required TResult Function( String ipAddr,  String netmask)  connected,required TResult Function()  disconnected,required TResult Function( String field0)  error,}) {final _that = this;
switch (_that) {
case ConnectionEvent_Connecting():
return connecting();case ConnectionEvent_Connected():
return connected(_that.ipAddr,_that.netmask);case ConnectionEvent_Disconnected():
return disconnected();case ConnectionEvent_Error():
return error(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  connecting,TResult? Function( String ipAddr,  String netmask)?  connected,TResult? Function()?  disconnected,TResult? Function( String field0)?  error,}) {final _that = this;
switch (_that) {
case ConnectionEvent_Connecting() when connecting != null:
return connecting();case ConnectionEvent_Connected() when connected != null:
return connected(_that.ipAddr,_that.netmask);case ConnectionEvent_Disconnected() when disconnected != null:
return disconnected();case ConnectionEvent_Error() when error != null:
return error(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class ConnectionEvent_Connecting extends ConnectionEvent {
  const ConnectionEvent_Connecting(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ConnectionEvent_Connecting);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ConnectionEvent.connecting()';
}


}




/// @nodoc


class ConnectionEvent_Connected extends ConnectionEvent {
  const ConnectionEvent_Connected({required this.ipAddr, required this.netmask}): super._();
  

 final  String ipAddr;
 final  String netmask;

/// Create a copy of ConnectionEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ConnectionEvent_ConnectedCopyWith<ConnectionEvent_Connected> get copyWith => _$ConnectionEvent_ConnectedCopyWithImpl<ConnectionEvent_Connected>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ConnectionEvent_Connected&&(identical(other.ipAddr, ipAddr) || other.ipAddr == ipAddr)&&(identical(other.netmask, netmask) || other.netmask == netmask));
}


@override
int get hashCode => Object.hash(runtimeType,ipAddr,netmask);

@override
String toString() {
  return 'ConnectionEvent.connected(ipAddr: $ipAddr, netmask: $netmask)';
}


}

/// @nodoc
abstract mixin class $ConnectionEvent_ConnectedCopyWith<$Res> implements $ConnectionEventCopyWith<$Res> {
  factory $ConnectionEvent_ConnectedCopyWith(ConnectionEvent_Connected value, $Res Function(ConnectionEvent_Connected) _then) = _$ConnectionEvent_ConnectedCopyWithImpl;
@useResult
$Res call({
 String ipAddr, String netmask
});




}
/// @nodoc
class _$ConnectionEvent_ConnectedCopyWithImpl<$Res>
    implements $ConnectionEvent_ConnectedCopyWith<$Res> {
  _$ConnectionEvent_ConnectedCopyWithImpl(this._self, this._then);

  final ConnectionEvent_Connected _self;
  final $Res Function(ConnectionEvent_Connected) _then;

/// Create a copy of ConnectionEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? ipAddr = null,Object? netmask = null,}) {
  return _then(ConnectionEvent_Connected(
ipAddr: null == ipAddr ? _self.ipAddr : ipAddr // ignore: cast_nullable_to_non_nullable
as String,netmask: null == netmask ? _self.netmask : netmask // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class ConnectionEvent_Disconnected extends ConnectionEvent {
  const ConnectionEvent_Disconnected(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ConnectionEvent_Disconnected);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ConnectionEvent.disconnected()';
}


}




/// @nodoc


class ConnectionEvent_Error extends ConnectionEvent {
  const ConnectionEvent_Error(this.field0): super._();
  

 final  String field0;

/// Create a copy of ConnectionEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ConnectionEvent_ErrorCopyWith<ConnectionEvent_Error> get copyWith => _$ConnectionEvent_ErrorCopyWithImpl<ConnectionEvent_Error>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ConnectionEvent_Error&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ConnectionEvent.error(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ConnectionEvent_ErrorCopyWith<$Res> implements $ConnectionEventCopyWith<$Res> {
  factory $ConnectionEvent_ErrorCopyWith(ConnectionEvent_Error value, $Res Function(ConnectionEvent_Error) _then) = _$ConnectionEvent_ErrorCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$ConnectionEvent_ErrorCopyWithImpl<$Res>
    implements $ConnectionEvent_ErrorCopyWith<$Res> {
  _$ConnectionEvent_ErrorCopyWithImpl(this._self, this._then);

  final ConnectionEvent_Error _self;
  final $Res Function(ConnectionEvent_Error) _then;

/// Create a copy of ConnectionEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ConnectionEvent_Error(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
