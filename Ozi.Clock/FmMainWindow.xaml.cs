using System;
using System.Reflection;
using System.Windows;
using System.Windows.Interop;
using System.Runtime.InteropServices;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;

namespace Ozi.Utilities;

public partial class FmMainWindow : Window
{
    private double _fTransparentValue = 90;
    private const int FoldedHeight = 29;
    private const int UnfoldedHeight = 62;
    private bool _fIsTransparent;
    public bool IsAutoFold;
    private bool _isFolded;
    public readonly FmSlider NewFmSlider;
    private readonly FmRulers _FmRulers;
    private DateTime _localTime;
    private DispatcherTimer? _timeTimer;

    // Windows API constants
    private const int HWND_TOPMOST = -1;
    private const int HWND_NOTOPMOST = -2;
    private const int SWP_NOMOVE = 0x0002;
    private const int SWP_NOSIZE = 0x0001;
    private const int SWP_NOACTIVATE = 0x0010;
    private const int SWP_SHOWWINDOW = 0x0040;

    public bool UseSnap = true;

    // P/Invoke declaration for SetWindowPos - required for making app always on top
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy,
        uint uFlags);

    // Constructor
    public FmMainWindow()
    {
        InitializeComponent();
        SetupWindowEvents();

        NewFmSlider = new FmSlider(this);
        _FmRulers = new FmRulers(this);
        
    }

    private void SetupWindowEvents()
    {
        // Handle when the window becomes visible after being hidden
        this.IsVisibleChanged += (s, e) =>
        {
            if ((bool)e.NewValue)
            {
                MakeWindowAlwaysOnTop();
            }
        };
    }

    private void MakeWindowAlwaysOnTop()
    {
        WindowInteropHelper wndHelper = new WindowInteropHelper(this);
        IntPtr hWnd = wndHelper.Handle;

        if (App.Settings.TopMost)
        {
            // Set window to be always on top
            SetWindowPos(hWnd, new IntPtr(HWND_TOPMOST), 0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
        else
        {
            // Remove always-on-top status
            SetWindowPos(hWnd, new IntPtr(HWND_NOTOPMOST), 0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
        Console.WriteLine(App.Settings.TopMost);
    }

    public bool IsTransparent
    {
        set
        {
            _fIsTransparent = value;
            if (!_fIsTransparent)
            {
                Opacity = 1;
            }
        }
        get { return _fIsTransparent; }
    }

    public double TransparentValue
    {
        set
        {
            _fTransparentValue = value;
            if (_fIsTransparent)
            {
                FmMain.Opacity = _fTransparentValue;
            }
            else
            {
                FmMain.Opacity = 1;
            }
        }
        get { return _fTransparentValue; }
    }

    private void fmMain_Loaded(object sender, RoutedEventArgs e)
    {
        init_Timer();
        read_Config();

        // _lstClock.Add(new OsClock("NYK", "Eastern Standard Time", "#FFAAAAFF", _lstClock.Count * 100));
        // _lstClock.Add(new OsClock("LDN", "GMT Standard Time", "#FFAAFFAA", _lstClock.Count * 100));
        // _lstClock.Add(new OsClock("KYIV", "FLE Standard Time", "#FFAAFFFF", _lstClock.Count * 100));
        // _lstClock.Add(new OsClock("PUN", "India Standard Time", "#FF99BBBB", _lstClock.Count * 100));
        // _lstClock.Add(new OsClock("SGP", "Singapore Standard Time", "#FFFFFFAA", _lstClock.Count * 100));
        // _lstClock.Add(new OsClock("TKO", "Tokyo Standard Time", "#FFFFAAAA", _lstClock.Count * 100));

        foreach (var timeZoneInfo in App.Settings.TimeZones)
        {
            var clock = new OsClock(timeZoneInfo.Value.Label ?? timeZoneInfo.Key,
                timeZoneInfo.Value.TimeZone,
                timeZoneInfo.Value.Color,
                App.Settings.LstClock.Count,
                timeZoneInfo.Value.IsMain ?? false);
            App.Settings.LstClock.Add(clock);

            gdMain.Children.Add(clock.OsGrid);
            if (timeZoneInfo.Value.IsMain ?? false)
            {
                App.Settings.MainClockIndex = gdMain.Children.Count - 1;
                Console.WriteLine(App.Settings.MainClockIndex);
                App.Settings.MainTimeZone = timeZoneInfo.Value.TimeZone;
                Console.WriteLine(App.Settings.MainTimeZone);
            }
        }

        FmMain.Width = App.Settings.LstClock.Count * 100 + 1;


        //Context menu
        var mainMenu = new ContextMenu();

        var item1 = new MenuItem
        {
            Header = "About"
        };

        item1.Click += MenuItemAbout_Click;
        mainMenu.Items.Add(item1);

        var item3 = new MenuItem
        {
            Header = "Settings"
        };
        item3.Click += MenuItemSettings_Click;
        mainMenu.Items.Add(item3);

        mainMenu.Items.Add(new Separator());

        var item5 = new MenuItem
        {
            Header = "Exit"
        };
        item5.Click += MenuItemExit_Click;
        mainMenu.Items.Add(item5);

        FmMain.ContextMenu = mainMenu;
    }

    private void read_Config()
    {
        Left = App.Settings.MainWndLeft;
        Top = App.Settings.MainWndTop;
        IsTransparent = App.Settings.IsTransparent;
        TransparentValue = App.Settings.TransparentValue;
        ShowInTaskbar = App.Settings.ShowInTaskBar;
        Topmost = App.Settings.TopMost;
        IsAutoFold = App.Settings.IsAutoFold;
        UseSnap = App.Settings.UseSnap;
    }

    private void save_Config()
    {
        App.Settings.MainWndLeft = Left;
        App.Settings.MainWndTop = Top;
        App.Settings.IsTransparent = IsTransparent;
        App.Settings.TransparentValue = TransparentValue;
        App.Settings.ShowInTaskBar = ShowInTaskbar;
        App.Settings.TopMost = Topmost;
        App.Settings.IsAutoFold = IsAutoFold;
        App.Settings.UseSnap = UseSnap;
        App.Settings.Save();
    }

    private void init_Timer()
    {
        _timeTimer = new DispatcherTimer
        {
            Interval = TimeSpan.FromSeconds(1)
        };
        _timeTimer.Tick += TtTick;
        _timeTimer.Start();
    }


    private void TtTick(object sender, EventArgs e)
    {
        MakeWindowAlwaysOnTop();

        if (NewFmSlider.Visibility == Visibility.Visible)
        {
            _localTime = NewFmSlider.CurTime.Date;
            _localTime = _localTime.AddMinutes((int)NewFmSlider.slTimeChecker.Value * 5);
            _FmRulers.rwTop.Height = new GridLength(_FmRulers.rwTop.MaxHeight *
                NewFmSlider.slTimeChecker.Value / NewFmSlider.slTimeChecker.Maximum);
        }
        else
        {
            _localTime = DateTime.Now;
        }

        foreach (var item in App.Settings.LstClock!)
            item.SetTime(_localTime);
    }

    private void MenuItemSettings_Click(object sender, RoutedEventArgs e)
    {
        var newFmSettings = new FmSettings(this);
        newFmSettings.ShowDialog();
    }

    private void AdjustTimeCheckerPosition()
    {
        _FmRulers.Left = Left;
        _FmRulers.Top = Top + Height;
        NewFmSlider.Left = Left;
        NewFmSlider.Top = Top + Height + _FmRulers.Height;
    }

    private void MenuItemTimeChecker_Click(object sender, RoutedEventArgs e)
    {
        if (NewFmSlider.Visibility == Visibility.Visible)
        {
            NewFmSlider.Hide();
            _FmRulers.Hide();
            _timeTimer!.Interval = TimeSpan.FromSeconds(1);
        }
        else
        {
            NewFmSlider.CurTime = _localTime;
            NewFmSlider.slTimeChecker.Value = _localTime.Hour * 12 + (int)(_localTime.Minute / 5);
            NewFmSlider.Show();
            _FmRulers.Show();
            _timeTimer!.Interval = TimeSpan.FromMicroseconds(100);
        }

        fmMain_MouseLeave(null!, null!);
    }

    private void MenuItemExit_Click(object sender, RoutedEventArgs e)
    {
        Application.Current.Shutdown();
    }

    private void MenuItemAbout_Click(object sender, RoutedEventArgs e)
    {
        var version = Assembly.GetExecutingAssembly().GetName().Version?.ToString();
        MessageBox.Show("Ozi clock. " + version);
    }

    private void fmMain_Closing(object sender, System.ComponentModel.CancelEventArgs e)
    {
        save_Config();
    }

    private void fmMain_MouseDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true; //double click  

        if (e.ClickCount > 1)
        {
            MenuItemTimeChecker_Click(null!, null!);
        }

        if (e.ChangedButton == MouseButton.Left)
        {
            FmMain.DragMove();
        }

        if (e.ChangedButton == MouseButton.Middle)
        {
            if (_isFolded)
            {
                UnFoldMainWindow();
            }
            else
            {
                FoldMainWindow();
            }
        }
    }

    private void FoldingMainWindow()
    {
        var i = (int)Height;
        if (IsMouseOver)
        {
            while (i <= UnfoldedHeight)
                if (IsMouseOver)
                {
                    Height = i++;
                    AdjustTimeCheckerPosition();
                }
                else
                {
                    return;
                }
        }
        else
        {
            while (i >= FoldedHeight)
                if (!IsMouseOver)
                {
                    Height = i--;
                    AdjustTimeCheckerPosition();
                }
                else
                {
                    return;
                }
        }
    }

    private void fmMain_MouseEnter(object sender, MouseEventArgs e)
    {
        if (IsTransparent)
        {
            Opacity = 1;
        }

        if (IsAutoFold && _isFolded)
        {
            FoldingMainWindow();
        }
    }

    private void fmMain_MouseLeave(object sender, MouseEventArgs e)
    {
        if (IsTransparent && !(NewFmSlider.Visibility == Visibility.Visible))
        {
            Opacity = TransparentValue;
        }
        else
        {
            Opacity = 100;
        }

        if (IsAutoFold && _isFolded)
        {
            FoldingMainWindow();
        }
    }

    private void FoldMainWindow()
    {
        {
            for (var i = (int)Height; i > FoldedHeight; i--)
            {
                FmMain.Height = i;
                AdjustTimeCheckerPosition();
            }

            _isFolded = true;
        }
    }

    private void UnFoldMainWindow()
    {
        {
            for (var i = (int)Height; i < UnfoldedHeight; i++)
            {
                FmMain.Height = i;
                AdjustTimeCheckerPosition();
            }

            _isFolded = false;
        }
    }

    private void fmMain_LocationChanged(object sender, EventArgs e)
    {
        if (UseSnap)
        {
            SnapWindow();
        }

        AdjustTimeCheckerPosition();
    }

    // private bool _isAlreadySnapped = false;

    private void SnapWindow()
    {
        if (!UseSnap)
        {
            return;
        }

        // Screen screenTmp = null;
        // foreach (Screen screen in screens)
        //     if (screen.WorkingArea.Contains((int)(this.Left + this.Width / 2), (int)(this.Top + this.Height / 2)))
        //         screenTmp = screen;

        // if (screenTmp == null) return;
        //
        // if (Math.Abs(screenTmp.WorkingArea.Left - this.Left) <= 20)
        // {
        //     this.Left = screenTmp.WorkingArea.Left;
        // }
        //
        // int borderLim = 0;
        // if (!isAlreadySnapped)
        // {
        //     borderLim = 20;
        // }
        // else
        // {
        //     borderLim = 45;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Top - this.Top) <= borderLim && !isAlreadySnapped)
        // {
        //     this.Top = screenTmp.WorkingArea.Top;
        //     isAlreadySnapped = true;
        // }
        // else
        // {
        //     isAlreadySnapped = false;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Right - (this.Left + this.Width)) <= 20)
        // {
        //     this.Left = screenTmp.WorkingArea.Right - this.Width;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Bottom - (this.Top + this.Height)) <= 20)
        // {
        //     this.Top = screenTmp.WorkingArea.Bottom - this.Height;
        // }
    }
}