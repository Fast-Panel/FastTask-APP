; Notifie Windows Shell que les icônes ont changé après installation/désinstallation.
; Equivalent à SHChangeNotify(SHCNE_ASSOCCHANGED, ...) — force le rafraîchissement
; du cache d'icônes sans redémarrage de l'explorateur.
!macro RefreshShellIcons
  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend
