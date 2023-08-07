
.PHONY: all emuiibo sysmodule overlay emuiigen dist emuiibo-clean emuiigen-clean

# We need to provide a custom target triple since the official tier 3 one doesn't provide crypto support
TARGET_TRIPLE := aarch64-nintendo-switch-freestanding-crypto
PROGRAM_ID := 0100000000000352

emuiibo: sysmodule overlay dist

sysmodule:
	@cd emuiibo && cargo update && cargo nx build --release --target $(TARGET_TRIPLE).json

overlay:
	@$(MAKE) -C overlay/

dist:
	@rm -rf $(CURDIR)/SdOut
	@mkdir -p $(CURDIR)/SdOut/atmosphere/contents/$(PROGRAM_ID)/flags
	@touch $(CURDIR)/SdOut/atmosphere/contents/$(PROGRAM_ID)/flags/boot2.flag
	@cp $(CURDIR)/emuiibo/target/$(TARGET_TRIPLE)/release/emuiibo.nsp $(CURDIR)/SdOut/atmosphere/contents/$(PROGRAM_ID)/exefs.nsp
	@cp $(CURDIR)/emuiibo/toolbox.json $(CURDIR)/SdOut/atmosphere/contents/$(PROGRAM_ID)/toolbox.json
	@mkdir -p $(CURDIR)/SdOut/switch/.overlays
	@cp $(CURDIR)/overlay/emuiibo.ovl $(CURDIR)/SdOut/switch/.overlays/emuiibo.ovl
	@mkdir -p $(CURDIR)/SdOut/emuiibo/overlay
	@cp -r $(CURDIR)/overlay/lang $(CURDIR)/SdOut/emuiibo/overlay/

emuiigen:
	@cd emuiigen && mvn package

emuiibo-clean:
	@rm -rf $(CURDIR)/SdOut
	@cd emuiibo && cargo clean
	@$(MAKE) clean -C overlay/

emuiigen-clean:
	@cd emuiigen && mvn clean

all: emuiibo emuiigen