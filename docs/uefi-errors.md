# UEFI-loader error codes

## EER 1: LoadedImage protocol

### Hvad loaderen prøver

Den kalder **handle_protocol(image_handle, LOADED_IMAGE_GUID)** og spørger firmaren:"Fortæl mig om det program, der kører lige nu (os selv)". Svaret indeholder **device_handle**, altså den disk vi blev bootet fra. Den skal vi bruge for at finde **KERNEL.BIN** på samme disk.

### Hvor sandsynlig er den? 

På en fungerende loader er det meget usandsynligt. Firmwaren installere altid LoadedImage på et program, den starter. Hvis fejl 1 sker, er det næsten altid koden som der er noget galt med. Ikke firmwaren.

### Typiske årsager

- **Tastefejl i LOADED_IMAGE_GUID**. Et forkert ciffer, og firmwaren kennder ikke protokollen. Den rigtige værdi fra UEFI-spec'en er "5B1B31A1-9562-11D2-8E3F-00A0C9693B3D" 
- **Forkert image_handle**. Hvis signaturen på efi_main er forkert, fx parametrene byttet om eller extern "efiapi" mangler, får vi skrald i stedet for vores handle.
- **Felterne i BootServices** står i forkert rækkefølge før handle_protocol. så kalder vi en helt anden funktion. Det giver oftere et crash eller en hængning end en pæn ERR 1, men det kan ske.

### Sådan undersøger man det

- Sammenlign LOADED_IMAGE_GUID ciffer for ciffer med spec'en.
- Tjek at efi_main er extern "efiapi" fn(image_handle, system_table) i den rækkefølge.
- Tæl felterne i BootServices op til handle_protocol og sammenlign med spec'en.

