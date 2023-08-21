
.PHONY: all dev emuiibo emuiibo-dev sysmodule sysmodule-dev overlay emuiigen dist clean emuiibo-clean emuiigen-clean

# We need to provide a custom target triple since the official tier 3 one doesn't provide crypto support
TARGET_TRIPLE := aarch64-nintendo-switch-freestanding-crypto
PROGRAM_ID := 0100000000000352

all: emuiibo emuiigen

dev: emuiibo-dev emuiigen

clean: emuiibo-clean emuiigen-clean

emuiibo: sysmodule overlay dist

emuiibo-dev: sysmodule-dev overlay dist

sysmodule:
	@cd emuiibo && cargo update && cargo nx build --release --target $(TARGET_TRIPLE).json

sysmodule-dev:
	@cd emuiibo && cargo update && cargo nx build --target $(TARGET_TRIPLE).json

overlay:
	@$(MAKE) -C overlay/

dist: sysmodule overlay
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