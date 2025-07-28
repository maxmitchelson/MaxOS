target_dir := "./iso_target"

iso_root_dir := target_dir / "iso"
boot_dir := iso_root_dir / "boot"
efi_dir := iso_root_dir / "EFI/BOOT"
iso_file := target_dir / "max-os.iso"

ovmf_dir := "./build/ovmf/"
ovmf_code := ovmf_dir / "OVMF_CODE.4m.fd"
ovmf_vars := ovmf_dir / "OVMF_VARS.4m.fd"

limine_dir := target_dir / "limine"
limine_repo := "https://github.com/limine-bootloader/limine.git"
limine_branch := "v9.x-binary" 

limine_conf := "./build/limine.conf"
sysfile := "limine-bios.sys"
bios_cd := "limine-bios-cd.bin"
uefi_cd := "limine-uefi-cd.bin"
x64_efi := "BOOTX64.EFI"
ia32_efi := "BOOTIA32.EFI"

@limine:
    if ! {{path_exists(limine_dir)}}; then \
        git clone {{limine_repo}} {{limine_dir}} --branch={{limine_branch}} --depth=1; \
    fi

    make -C {{limine_dir}} --silent

clean:
    rm -rf {{target_dir}}

@run binary_file: limine
    mkdir -p {{boot_dir / "limine"}}
    cp -n {{limine_dir/sysfile}} {{limine_dir/bios_cd}} {{limine_dir/uefi_cd}} {{boot_dir / "limine/"}}
    cp {{limine_conf}} {{boot_dir / "limine/"}}

    mkdir -p {{efi_dir}}
    cp -n {{limine_dir/x64_efi}} {{limine_dir/ia32_efi}} {{efi_dir + "/"}}

    cp {{binary_file}} {{boot_dir + "/"}}

    xorriso -report_about "SORRY" as mkisofs -R -r -J -b boot/limine/{{bios_cd}} \
        -no-emul-boot -boot-load-size 4 -boot-info-table -hfsplus \
        -apm-block-size 2048 --efi-boot boot/limine/{{uefi_cd}} \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        {{iso_root_dir}} -o {{iso_file}}

    {{limine_dir / "limine"}} bios-install --quiet {{iso_file}}

    qemu-system-x86_64 \
        -M q35 \
        -m 1G \
        -drive if=pflash,unit=0,format=raw,file={{ovmf_code}},readonly=on \
        -drive if=pflash,unit=1,format=raw,file={{ovmf_vars}} \
        -cdrom {{iso_file}}
