import AppKit
import ScreenCaptureKit

// Host-only menu contract. The original engine menu can emit this same selection
// without changing the live driver, save lifecycle or campaign continuation.
struct CampaignSelection {
    let mission:String
    let difficulty:UInt32
    var difficultyTitle:String { ["Fácil","Normal","Difícil"][Int(difficulty)] }
    /// Start in the original main menu (MenuInit 2) instead of a stage.
    static let menu="MENU"
    var isMenu:Bool { mission==Self.menu }
}

final class NativeCampaignMenu:NSObject,NSWindowDelegate {
    let window:NSWindow
    let stage=NSPopUpButton(frame:.zero,pullsDown:false)
    let variant=NSPopUpButton(frame:.zero,pullsDown:false)
    let difficulty=NSPopUpButton(frame:.zero,pullsDown:false)
    let start=NSButton(title:"Iniciar fase",target:nil,action:nil)
    let cancel=NSButton(title:"Voltar ao jogo",target:nil,action:nil)
    let catalog:[Int:[String]]
    var selected:((CampaignSelection)->Void)?
    var cancelled:(()->Void)?
    init(root:URL) throws {
        let data=try Data(contentsOf:root.appendingPathComponent("native-inputs/manifest.json"))
        guard let manifest=try JSONSerialization.jsonObject(with:data) as? [String:Any],let files=manifest["files"] as? [String:String] else{throw NSError(domain:"Campanha",code:1,userInfo:[NSLocalizedDescriptionKey:"Manifesto de fases inválido."])}
        let names=files.keys.filter{$0.range(of:"^MONDE[0-9]{2}[A-D]?\\.ETA$",options:.regularExpression) != nil}.sorted()
        guard names.count==38 else{throw NSError(domain:"Campanha",code:1,userInfo:[NSLocalizedDescriptionKey:"Os 38 arquivos de fase não estão disponíveis. Recompile o aplicativo."])}
        catalog=Dictionary(grouping:names,by:{Int($0.dropFirst(5).prefix(2))!})
        guard catalog.keys.sorted()==Array(1...25) else{throw NSError(domain:"Campanha",code:1,userInfo:[NSLocalizedDescriptionKey:"Catálogo de fases incompleto."])}
        window=NSWindow(contentRect:NSRect(x:0,y:0,width:580,height:370),styleMask:[.titled,.closable,.miniaturizable],backing:.buffered,defer:false)
        super.init()
        window.title="Rising Lands · Campanha";window.delegate=self
        let title=NSTextField(labelWithString:"Rising Lands")
        title.font = .systemFont(ofSize:30,weight:.semibold);title.frame=NSRect(x:30,y:300,width:520,height:42)
        let subtitle=NSTextField(wrappingLabelWithString:"Escolha uma fase e a dificuldade para começar.")
        subtitle.frame=NSRect(x:30,y:253,width:520,height:35)
        window.contentView?.addSubview(title);window.contentView?.addSubview(subtitle)
        for (caption,control,y) in [("Fase",stage,210.0),("Versão",variant,164.0),("Dificuldade",difficulty,118.0)] {
            let label=NSTextField(labelWithString:caption);label.frame=NSRect(x:30,y:y+4,width:110,height:22)
            control.frame=NSRect(x:145,y:y,width:405,height:28)
            window.contentView?.addSubview(label);window.contentView?.addSubview(control)
        }
        for number in catalog.keys.sorted(){stage.addItem(withTitle:String(format:"Fase %02d",number));stage.lastItem?.tag=number}
        stage.target=self;stage.action=#selector(changeStage)
        difficulty.addItems(withTitles:["Fácil","Normal","Difícil"]);difficulty.selectItem(at:1)
        start.target=self;start.action=#selector(begin);start.keyEquivalent="\r";start.frame=NSRect(x:390,y:35,width:160,height:32)
        cancel.target=self;cancel.action=#selector(back);cancel.frame=NSRect(x:200,y:35,width:175,height:32)
        window.contentView?.addSubview(start);window.contentView?.addSubview(cancel)
        changeStage();window.center()
    }
    @objc func changeStage(){
        variant.removeAllItems()
        for file in catalog[stage.selectedItem!.tag]! {
            let suffix=String(file.dropFirst(7).dropLast(4))
            variant.addItem(withTitle:suffix.isEmpty ? "Original · \(file)":"Variante \(suffix) · \(file)")
            variant.lastItem?.representedObject=file
        }
        variant.isEnabled=variant.numberOfItems>1
    }
    var selection:CampaignSelection {
        CampaignSelection(mission:variant.selectedItem!.representedObject as! String,difficulty:UInt32(difficulty.indexOfSelectedItem))
    }
    func show(canReturn:Bool){cancel.isHidden = !canReturn;window.makeKeyAndOrderFront(nil)}
    @objc func begin(){selected?(selection)}
    @objc func back(){window.orderOut(nil);cancelled?()}
    func windowShouldClose(_ sender:NSWindow)->Bool{back();return false}
}

// Capture only this process's window. AppKit's cached bitmap omits compositor
// layers of modern popup controls, so it cannot verify their visible text.
func captureOwnWindow(_ window:NSWindow,to url:URL,completion:@escaping(Bool)->Void){
    if #available(macOS 14.4,*) {
        SCShareableContent.getCurrentProcessShareableContent{content,error in
            guard let own=content?.windows.first(where:{$0.windowID==CGWindowID(window.windowNumber)}) else{DispatchQueue.main.async{completion(false)};return}
            let config=SCStreamConfiguration();config.width=Int(window.frame.width*2);config.height=Int(window.frame.height*2);config.showsCursor=false
            SCScreenshotManager.captureImage(contentFilter:SCContentFilter(desktopIndependentWindow:own),configuration:config){image,error in
                var passed=false
                if let image=image,let data=NSBitmapImageRep(cgImage:image).representation(using:.png,properties:[:]){do{try data.write(to:url,options:.atomic);passed=true}catch{}}
                DispatchQueue.main.async{completion(passed)}
            }
        }
    }else{completion(false)}
}
