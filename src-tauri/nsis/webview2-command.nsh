; FyAgent secure WebView2 bootstrapper command.
; Deterministic gzip of UTF-16LE source; verifier checks exact compressed bytes.
!define FYAGENT_WEBVIEW2_COMMAND_CHUNK_COUNT 10
!define FYAGENT_WEBVIEW2_LOADER_BASE64 "dAByAHkAewBmAG8AcgBlAGEAYwBoACgAJABpACAAaQBuACAAMAAuAC4AOQApAHsAJABlACsAPQBbAEUAbgB2AGkAcgBvAG4AbQBlAG4AdABdADoAOgBHAGUAdABFAG4AdgBpAHIAbwBuAG0AZQBuAHQAVgBhAHIAaQBhAGIAbABlACgAIgBGAFkAXwBXAFYAMgBfACQAaQAiACkAfQA7ACQAYwA9AFsAYgB5AHQAZQBbAF0AXQBbAEMAbwBuAHYAZQByAHQAXQA6ADoARgByAG8AbQBCAGEAcwBlADYANABTAHQAcgBpAG4AZwAoACQAZQApADsAJABtAD0AWwBJAE8ALgBNAGUAbQBvAHIAeQBTAHQAcgBlAGEAbQBdADoAOgBuAGUAdwAoACQAYwApADsAJgAoAFsAUwBjAHIAaQBwAHQAQgBsAG8AYwBrAF0AOgA6AEMAcgBlAGEAdABlACgAWwBJAE8ALgBTAHQAcgBlAGEAbQBSAGUAYQBkAGUAcgBdADoAOgBuAGUAdwAoAFsASQBPAC4AQwBvAG0AcAByAGUAcwBzAGkAbwBuAC4ARwBaAGkAcABTAHQAcgBlAGEAbQBdADoAOgBuAGUAdwAoACQAbQAsAFsASQBPAC4AQwBvAG0AcAByAGUAcwBzAGkAbwBuAC4AQwBvAG0AcAByAGUAcwBzAGkAbwBuAE0AbwBkAGUAXQAwACkALABbAFQAZQB4AHQALgBFAG4AYwBvAGQAaQBuAGcAXQA6ADoAVQBuAGkAYwBvAGQAZQApAC4AUgBlAGEAZABUAG8ARQBuAGQAKAApACkAKQB9AGMAYQB0AGMAaAB7AGUAeABpAHQAIAAxAH0ACgA="
!define FYAGENT_WEBVIEW2_COMMAND_00 "H4sIAAAAAAAC/9Vda3MaydV+PutXTMlKAVkx1v3mUiUIoViJtVJJ9m7ekp0sgpFEjIAwYFvZ+L+n+vTp6evMgIS9+5bLLmC6+1z69Ln3eIQ2xmjjAVUsIUKEa/yENvrooYs2JkhwhQQTVFHBKQZIMaHnfVSwiggV/IQEY/Rwi0ecY4A+HlFBDR+yFcUcMWKAO3zACs4wRBcJIhzSCv66S7lzTWgXhOE9llCjGStIMMAnHOACVwxlij6SbKSEuIwVGvEa5zhDC++tsSmWaS13jQammGCINxiijS7jdIExEtwSVgJ6x6DrRwwxQIIKlnCGHjoYY4gUQ9xighgXGOIzzbzCPRKiPkYTQ1rrPU7xgBF9m6Bu4RLhF+ZQHT/S/iUz0PV+DizO0MYAbdwhodUHmDx7fowRUnSxjmWLghOisePQ1cKYII3RQAcT9IibEa5oF0b/D3kqTlIHU5LfCR6fNff3wEupGeq0ypjmT4yzXeezmmbQ3rBGEWdanLAwVkUnSuFb4fOeGpCP0aMZHRozxiOu0EUX/Wz2OQ5whAaOSUNU0cArnOMUTZziFU7ou/hzhf9DreC5WKNGONgYnKBHUjEL3Dx4eXCkbpT70S2l9c+ZPpfQBYRFUb2as3IDp99w9bKZZZCfivfTVl+klOWt/C34PQvez+XIYvgtbb57LsInsOw0zHMWy6VoEavlzciDMC9e8632HN1VLgWLWG1xFD6NX8oDfUFWLkEbt4jIco3Jrt2ShVDWLyLd/Qk9JCy3"
!define FYAGENT_WEBVIEW2_COMMAND_01 "EW7JEj4gIj81oV96+MJPfesc4WckuMFPvMoGzfhMnvsAHxHhhiy2OBkxY3aEIXvBHdyjTSMjsiAa6oh+HRTAbWYW/go93GHAnnCECzTRQIQNrGEDW4jwEQn5Kv5JVXMFh5pBPl0RjhvYxk7gHDdxhH3sooU9+rvNn/axiQ2s4wRH2MAOtrBDv25jF1to4QTHWMcudrCDfRxhE0dYwzb2sI5jwlrM2kADTeyiiT00sIENmr2JTfq2hS2Ofl5k1O6gjjXs0b/7GZYtrGEd69hEC8fYwwm20UALR2hiC3s4pn9b2MYmTgijfaxjA/uE5RaOcUwzmzgh3rZo/D4aJJG7OMYRtgiCGCXgbxLGYm4Ta6gQHjaWYnSdvod1qNpfe3cvaF+uMMJH9Er2ZhtrtBuCU9vYxwbh3qDPTcZ6HztYxzaOsUO7sUuYN2lHxT61iCsbvBuS52vEtSbtr3gmKBb7uEeriNk7OMa+cR5vMSUPUnu8f8l8Vk3BayT4ggi/MgUjLyqWsehF9ntCEjpGleMcGStLT0zyY4X8wikSIxKWq9zgkWZf4wNFtEf8PeVRtSz6Fd6ly+frQCTQJLgjwuCOMBzhnp6IuKXB8z/gAAc0NuETViVYEZ1+ifmvBqZjolLAGRDcI4LXJD5+4hMr/pXrvsUwiwOEvJic82mRUYSIxKa0ymu06Xm1kD81Y80aYlwiwQh9tEnTigxFPctKVLLRX1mXDljOXTpdzIRvnVKMmFo8+ool+huSKpsrghd1HNO3"
!define FYAGENT_WEBVIEW2_COMMAND_02 "NxTD3LHe/T4S1qNoW/DOhO5KV49sVNUZJeK2Pun5NfosIEaBMXc85gt2cWv9qTn8FTPGpF3Evgh9domIvDS9Xo+s4pCkIaXcU2LYpBRTjLKoXVqmMXFFZBdiOu9qh2aj7Av2sOZh6kq8LYl/tqTT5W7NwWGFY1epS7vZ6RWS0ifqtfykiEkvDTi31UGMN8STiQVRnrQB2Xstl59xz15vmGa1UzVP7m0MY87DSRmuYo1Okgk/tPoNS6fkqtz/mgXFnnEYlKaUZCTCnsPF+Xakmu2sxEzJrktnkyRNnJEaceWH4ChxihuUoWjjkfn9tcSqXCIlKznFDf7Fe3xB3/q0r38jf0jw+Zb0pandv49umMd6/J0s+X6Of5aWjtigUxJ+lq+NwuPjAB9jnJOuiDlnPeW8U4fOiNA264ixgRh75D3E7IttYwv79E39rRRqrWWyNLa/HvKMlf8rvd6IcLqjfenReg+ZphuQJx7hEldo4IB27Pm01yhzbZ4f35r6OJ2SxA8yqAmdGtuni/h8b7JWEJ/X0DU+7xif97PPG2hnn/eMMcKLtX+34dxiNwfO+gyft43Pa4akhTlyQ5wwfZcj0gKPQS6Y+saFIDWJD2GeXW1ZWkj+qndY+DxtfMYxrdGegyZNi0/BZoa7TeusXk3d+V7E0zjokygdnD8vn1Jzr2aX7zx4eXDSGXV6Pp832QYvls+zcdXkoW1Ti+KhuuF7R3PwoNxOnuMzezvH5L33jYzd97GDbl1R1xLdGKxHUB54"
!define FYAGENT_WEBVIEW2_COMMAND_03 "xedW0BQHTrNV6+TrSUqE9Opa5Yrx2azs2BazauAYo4EJU3bDsVXq+GnXOMU54ixXGpohPU0ZX7WphpMQlT32msSK0saulVjOBvnvyudIgnmrUUaltI5tllINOyLvX0I/sDiz7PndqeNbi30r9oBE9alDdKd8FidEQb/w2ZUDR/JMS7b2Qb8fdHN86LSbJ2lRktyg8zubHC9lsbuc7XLjmHDv0PcRn2WlGaT8P5C8u3tc5Js3iBsJV3zNWqUL/bfWPbrv4VtBU9as4eRiJYdD+SezjjKL9q4X7Lv28cPwle8ufOMOS7vMUqcWNovUOPeUeYqIqxHJjrB9I5Yv6esPjTMtY8pjyg+/cXRRxLKpsFyeIXNkS6d/JmXXyX3mwXSy7PvvzU6mnC9us75J5raYfn+E1jFhLlwFYP5i4D+7TtKy6dMRk9aQ31PD9glt/kgckrY33POi+oYeaAW573EAcxOG1Oa6E6tY4n35FnZnSLm0CUf0I5bffHlKLRp7fOZm4YjQ574dDle/lLyF1yyrCbl7NaA+HKk5Evy7AG5xVnIeDiqNMSBvPzVwzqv4FWUo82bYOSQpcwIfKUUdkuEKqvgTDvAP/BereI8Uf0QN5zjMrduNOYuqJVGusIr/YoUqmYuTNJszdt5jdvx0bmQ2TtlaV+diP2KKFr7QnIHRkySlUcmRHH/LHU5t5nOVVvDnyvppPmY+vNTjr5YEH0JefktKeoUyW9uU4RLZrE3sBvZvNvp92PYKN8yPj8bvX3Nl"
!define FYAGENT_WEBVIEW2_COMMAND_04 "OnQq8zBY3Lm05c0+pSKrck/aucM2/W+co3tHGds74mxoByrBvL65llzJXOfwG2ZaWyXQfQ7bVYOlGaTCzTKtWrNuSRZTK4frSnIxjnEpFakn83leoczxbiKm2t06nwjxd5d/3yzRapWFabU221aZffpP1tkxZNugLa1ZWbdlzPWmlI9kV4HnydrkaadQJlHv4fO6Ncr8+DBt38r25Hki9u65PTkHhbj6VkZ0WbzliqXErE86c8B7P2A8p1nXpoxTVC0wNSqeose8Q3twxlaoxz6n2bkjM37ySUy+XUT+mKK6nenxFySdXZLHNslkm6oUEfFX0i/7gJqkscbsH77LVvvM+VRZo9UUyXFu/5LkrKTtE/O7S7HKmCiTGYpXBnYTomtiYfRA1ExojxLOj0bcP9zm3GNKFlryNWVMhnw2zVsFau4X1sraE9f+qubs4TetlSk4tmY2Y7D8rgwTz9hY64LkTUB85M6ITyxDilLz1oVt3b8VpUVYqGzZgDvWkoXQeELdIHe/MY0KC0ljiyMtcaY0LfNSq++99IKw0u9IczkuknJx/0X9Ni+97zBGn7gr8w9Sj/TxFj2KrZUekidV/XpF2RB1sk5YJ0kuDCnzkaLKlRjfB69ntsDG8QhTypV3C+KQWo7nrdcxI+aE9+tpHSF2TibcFaL+hGIZN9IfFOyKHef7FJZRKfIfXeKcn0sy8XAzCjaUrwV81bdINE9tSII/f+X6QTWAv+oZW52btlCPiDnb/V7m0dh+S5eiP+2h"
!define FYAGENT_WEBVIEW2_COMMAND_05 "3GRyGGW9u0/1Fw4KuLgciPIkL0YE5YJmnGWZCN3f5EcHkWGRZaVqQPr/C89YxyuusunfVZ9Wnky26LSoDFtqdfT4q/3AcPIiY3MtRcVscK8tSB9obH7Wyuw3zOuiXUQfUT0HC59aG68833/+fuA6VaFCfr8/Ol+nlMua8pf82aF8hSvNX0ssQD78EM5lEaUbkehT2cWQ7YGCbfNO10PLO5BUjFl3Yky3N0nHnHYmp6xT1jwX5X2yWqLMW4Eqe5V3b1jDVPND+vwUKX7kPNM5xviZdN4k8wFkR3D4vnCtJB8QwkzuwL9JA4+zqnpodT9n9PQaj11NC999jjiu6XGfp+K+jkdVBX3MN1fN+tOnLMsiq2BjjowHnBNSI7VE6Fi2nenfKKvOtinv58deEh8//nIzde9wiTdkj0eZvkuJijH92qcsy4AksU743mGaabTVrEKn7pnW8ZBxU94O7xEfBoYWfEGzZHw55u5jeWclL8MgvVN1v/OeOijEfdwDvMRL6j+K8RCoe3XIH31p3dR5iRFe4k/k34nvol/pkO44iFsPu9SfVSnEpuFwITW6kCp4Sbqgz8+Ur/PS2aMK3wURO/iFvGmx3lEORDPDJCCJezZnOKIVpJzdce1R5Zcus6j+0OhDuKboyN8TecalNTwh/7hLcJXUV3Nn/kDnX8hYjygzZ6sbEGIPHmhWgyjqO3GMwrnG90gWoYeKuVIzeu21DVKdOopqDVH9ouYnHJuUQTH1nmuxpCcjvqdcjZmNSyqP"
!define FYAGENT_WEBVIEW2_COMMAND_06 "NiUt8YnsQ5/zM6r29dXQRz2+AddmeA8kkb7GqFMH0D37K2PWLF3v5rOC3yXvpkf33sbs80iNNcgyXPec80qyiLzLmP2CP9DNc597f8B7nOARDcpMy96XMXubMur8hfsHukFr3iU6En4+ZIs+IMwEpXckJY+kQ00oEq8R62RZS44CN7+VFxhbd3t0xs6+UThgPkpJmVh9Dt1A943uyZI1bCEDErPbTNPeGxJwx7YkzSRAd1u8Kumz6HAk3DPqU0OOhEKYddkaTjij2aPII87u5ctagv3uDxUH2pa1TGctO3tT9zy8OlUpr/EX8hC6WQZEPFW/ValS7t+UEm8KqdDpXC7U87NQEqK7EvA5WujSKJeOK+4XGyFmay1tz2fOIenucLNqusKS1y18Lr3FAfsj5WMklxLak9B4ZXdf8+nr50JXI5skXXasZ47rZDWpPumwSW6deMXJZYdGKK+sab3vRt68XPKyuysB3eZ3qB0+oYvwuHTdcA5pFoxilpind/LN8gaPmlEXmzh1Vmkj8/Wim2X3T0ho9di5KTkLL2pBf7+447D89NZzu+VmfyuIriveZm9IWIRkneSslidPedAXKUXuOyA0dF+Hub3QpsbJr5uX6edVq4POhmDXP0wZk7Zi9RndwpoK3ZV2SXmAe8JWwvw540IRnjJHM84wVe+yMufsYJturu8UrnROO2f3KWsc3rIXOiUs3U6DsLS4vYfXVIGRVveaaga3Tj49zs7kA3klSkLVu7zM/a04XX0/kmTG"
!define FYAGENT_WEBVIEW2_COMMAND_07 "eM02RHir9puNDrGFmO6Xyb+rXEftZ71th8S7KctLn0b4+TzhGXwk63SIG4r7brFNd6Fu6eZal+ovbWyineUY9L0mhecV4faJ1jW79s3uRBULuZy9YK9wyL6Y2zv/bWCE+uRNSEWz31I1K8luoQsPUETO6swX+QjX3t7qT6anYM/N021hSLGhvVUn6CVpa62rw/nM/BXPKEZ3+0o73rrmDYgI2966vjc0K0dcmxrGdNXpLC6CHz+psndGcqdurVSxYUAp9uWuWfMkxrv8hM0Pz9En84rwk/dw8izE07HW2Of5y2HeyUyJ0HGPhP9s9kpmsVaD5y60//KNDX2OngVmpm7Xd4ZMvF8bcUFKT9t0n9POKoe5Hht8d2+ONPCZIj7V2a6fSAyk9q0a+lHbCJ+7siMtzTqxp4aVNat+ypOvel1vJj2h/kcf4iXnk2We8iyLmu8482trxEWs6j59R1o1BGsR6wrd3cnyN+a97GseISVGfjbHvs7yqfom42K698ae5pWUCkvyKbvtYb+F4jXe4i29AzLUDapqNfIkht514POyac2JnTPiPvdvdpbJ3ICrfUW41VlT6zWqpTP6fOPPfC9I8Yw7rqXNm1WuLXznbZ66N2mL3z9iR1nF7x6ZLYtRLhNKUzZYA6kVTB3/NG24lFmFKb2vw/SH7FvPpnXTvr7WqH3CWfZyTIjvglNubWAt+J6Sqqdv2kZlt4iHNm98e+dT5/Z1rHm/2OPt29Srgd6TWe3V82xWreT9LWGe/2Dw"
!define FYAGENT_WEBVIEW2_COMMAND_08 "sG2dcF2PDc98znmdvyKef1ZF3kxwN8nenaNOoo1XFMghp9x5Ha5w+3mA2IhFq57cRBzNaW7Wcm895PFU2uq1BeqyLt9I7HP0GpFcyY7bB/LIHoOayab7hOq+8q1bIS/dHp1X7S/LDD8vH1aeCZ89O+bmhcy+bT/robWMrJXI/mNVXR1bbx7T3UPSKj6QHTEjDlP3he5Jjozex8iyzC+8rulwFV1V5afsbel3Z6k3WZvvk7zJJDS27Jadwf+ts2PnXDcrmmHmw3QE4sYYZdmtS0tX1hbct1HGETv7LKDZbxO5ptPXJqkZ0igBK6XKsCkZV978vFyFD0nzRr/7/PCZ2Me5nQnz9TLU8S/OMEX0RstKIcx37DGoe8Ut47QkuX2K4bXs/OzQqpLnZW1CdaFZdzBvv+w148BO63u17pOlgg63vFWrpT1S89grlyNuX6vGOWS1XCx/Zh9K5v9bXGMzuRWuu/krtZxxS9QB1zFu2P6a+bpN1q/yVo/q7Fc9s6YX8YY7bE2tuGxYuvAbCfK5Z3dGmFZF1bz1bSVhyf/JlHXIRmnf1I7Z9T3ZGtOd1/kXvtepYkzTavgeqvm0uGOwHJLpZ/iQZvNWvs5MU170EaYxP1ZZLCbl0J8LMZRf9KGGs5DPg5wXVdUCXah58dfTMFDZtHBevJ719hRzzM595/PMzeY/j2u2TvOhujqvDNpz3wp0Sd9kN6X5lqtZ38pR7vHbK4X+v5Oy//NEd0MKTacyLzInn/xO+BDqBPj2lMvO"
!define FYAGENT_WEBVIEW2_COMMAND_09 "Yd1N7NvSJfwPyr0TTzRqAAA="

!macro FyAgentSetWebView2CommandEnvironment
  StrCpy $7 1
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_0", w "${FYAGENT_WEBVIEW2_COMMAND_00}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_1", w "${FYAGENT_WEBVIEW2_COMMAND_01}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_2", w "${FYAGENT_WEBVIEW2_COMMAND_02}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_3", w "${FYAGENT_WEBVIEW2_COMMAND_03}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_4", w "${FYAGENT_WEBVIEW2_COMMAND_04}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_5", w "${FYAGENT_WEBVIEW2_COMMAND_05}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_6", w "${FYAGENT_WEBVIEW2_COMMAND_06}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_7", w "${FYAGENT_WEBVIEW2_COMMAND_07}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_8", w "${FYAGENT_WEBVIEW2_COMMAND_08}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_9", w "${FYAGENT_WEBVIEW2_COMMAND_09}") i .r0'
  ${If} $0 == 0
    StrCpy $7 0
  ${EndIf}
!macroend

!macro FyAgentClearWebView2CommandEnvironment
  StrCpy $6 1
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_0", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_1", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_2", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_3", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_4", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_5", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_6", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_7", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_8", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
  System::Call 'kernel32::SetEnvironmentVariableW(w "FY_WV2_9", p 0) i .r0'
  ${If} $0 == 0
    StrCpy $6 0
  ${EndIf}
!macroend
