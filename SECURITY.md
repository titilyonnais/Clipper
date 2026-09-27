# Sécurité

Merci de ne pas signaler une vulnérabilité dans une issue publique. Utilisez
[« Report a vulnerability »](https://github.com/titilyonnais/Clipper/security/advisories/new)
(signalement privé GitHub). Seule la dernière version publiée reçoit des correctifs.

Pour vérifier un installateur téléchargé, comparez son empreinte avec le fichier
`SHA256SUMS.txt` de la release :

```powershell
Get-FileHash .\Clipper_2.0.0_x64-setup.exe -Algorithm SHA256
```
