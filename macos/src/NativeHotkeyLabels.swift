import AppKit
import Carbon

/// Resolve physical positions with the current macOS keyboard layout.
enum NativeHotkeyLabels {
    static func modifiers(_ flags:NSEvent.ModifierFlags) -> NativeHotkeyBindings.Modifiers {
        var result:NativeHotkeyBindings.Modifiers=[]
        for (flag,value):(NSEvent.ModifierFlags,NativeHotkeyBindings.Modifiers) in [(.shift,.shift),(.control,.control),(.option,.option),(.command,.command)] {
            if flags.contains(flag) { result.insert(value) }
        }
        return result
    }
    static func label(_ code:UInt16) -> String? {
        let special:[UInt16:String]=[36:"Enter",76:"Enter numérico",53:"Esc",49:"Espaço",123:"←",124:"→",125:"↓",126:"↑",117:"Delete",51:"⌫",48:"Tab"]
        if let label=special[code] { return label }
        guard let source=TISCopyCurrentKeyboardLayoutInputSource()?.takeRetainedValue(),
              let pointer=TISGetInputSourceProperty(source,kTISPropertyUnicodeKeyLayoutData) else { return nil }
        let data=Unmanaged<CFData>.fromOpaque(pointer).takeUnretainedValue()
        guard let bytes=CFDataGetBytePtr(data) else { return nil }
        let layout=UnsafeRawPointer(bytes).assumingMemoryBound(to:UCKeyboardLayout.self)
        var dead:UInt32=0,length=0,characters=[UniChar](repeating:0,count:8)
        let status=UCKeyTranslate(layout,code,UInt16(kUCKeyActionDisplay),0,UInt32(LMGetKbdType()),OptionBits(kUCKeyTranslateNoDeadKeysBit),&dead,8,&length,&characters)
        guard status==noErr,length>0 else { return nil }
        return String(utf16CodeUnits:characters,count:length).uppercased()
    }
}
