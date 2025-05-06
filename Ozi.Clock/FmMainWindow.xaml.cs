using System;
using System.ComponentModel;
using System.Linq;
using System.Reflection;
using System.Reflection.Metadata;
using System.Windows;
using System.Windows.Interop;
using System.Runtime.InteropServices;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using System.Windows.Media.Animation;

namespace Ozi.Utilities;

public partial class FmMainWindow : Window
{
    //Params related to force to topmost
    private const int HWND_TOPMOST = -1;
    private const uint SWP_NOSIZE = 0x0001;
    private const uint SWP_NOMOVE = 0x0002;
    private const uint SWP_NOACTIVATE = 0x0010;

    private const int FoldedHeight = 29;
    private const int UnfoldedHeight = 62;
    private bool _isFolded;
    public readonly FmSlider fmSlider;
    private readonly FmRulers fmRulers;
    private DateTime _localTime;
    private DispatcherTimer? _timeTimer;
    private bool isMouseOver = false;
    private bool isWindowFocused = false;

    // P/Invoke declaration for SetWindowPos - required for making app always on top
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy,
        uint uFlags);


    // Constructor
    public FmMainWindow()
    {
        InitializeComponent();
        // CommonTimeZones.ShowTimeZones();
        fmSlider = new FmSlider(this);
        fmRulers = new FmRulers(this);

        // Subscribe to changes in the App.Settings.Opacity
        App.Settings.PropertyChanged += (s, e) =>
        {
            if (e.PropertyName == nameof(App.Settings.Opacity) && !isMouseOver)
            {
                Opacity = App.Settings.Opacity;
            }
        };

        // Initial opacity from settings
        Opacity = App.Settings.Opacity;
    }

    private void fmMain_LocationChanged(object sender, EventArgs e)
    {
        fmRulers.Left = Left;
        fmRulers.Top = Top + Height;
        fmSlider.Left = Left;
        fmSlider.Top = Top + Height + fmRulers.Height;
    }

    private void fmMain_Loaded(object sender, RoutedEventArgs e)
    {
        init_Timer();
        read_Config();

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
                App.Settings.MainTimeZone = timeZoneInfo.Value.TimeZone;
            }
        }

        App.Settings.PropertyChanged += (sender, e) =>
        {
            if (e.PropertyName == nameof(App.Settings.Opacity) && !isMouseOver)
            {
                AnimateOpacity(App.Settings.Opacity);
            }
        };
        CreateContextMenu();
        this.Activated += (s, e) =>
        {
            isWindowFocused = true;
            UpdateOpacity();
        };

        this.Deactivated += (s, e) =>
        {
            isWindowFocused = false;
            UpdateOpacity();
        };
    }

    private MenuItem itemMoveLeft;
    private MenuItem itemMoveRight;
    private MenuItem itemMakeMain;
    private MenuItem itemRemove;

    private void CreateContextMenu()
    {
        var mainMenu = new ContextMenu();
        mainMenu.Opened += ContextMenu_Opened;

        var itemClock = new MenuItem { Header = "Clock" };
        mainMenu.Items.Add(itemClock);

        var itemEdit = new MenuItem { Header = "Edit" };
        itemEdit.Click += ItemEditClick;
        itemClock.Items.Add(itemEdit);

        itemMoveLeft = new MenuItem { Header = "Move Left" };
        itemMoveLeft.Click += ItemMoveLeftOnClick;
        itemClock.Items.Add(itemMoveLeft);

        itemMoveRight = new MenuItem { Header = "Move Right" };
        itemMoveRight.Click += ItemMoveRightOnClick;
        itemClock.Items.Add(itemMoveRight);

        itemMakeMain = new MenuItem { Header = "Make as Main" };
        itemMakeMain.Click += ItemMakeMainOnClick;
        itemClock.Items.Add(itemMakeMain);

        itemRemove = new MenuItem { Header = "Remove" };
        itemRemove.Click += MenuItemRemove_Click;
        itemClock.Items.Add(itemRemove);

        var itemAbout = new MenuItem { Header = "About" };
        itemAbout.Click += MenuItemAbout_Click;
        mainMenu.Items.Add(itemAbout);

        var itemFold = new MenuItem { Header = "Fold/Unfold" };
        itemFold.Click += MenuItemFold_Click;
        mainMenu.Items.Add(itemFold);

        var itemShowRulers = new MenuItem { Header = "Show/hide rulers" };
        itemShowRulers.Click += MenuItemShowRulers_Click;
        mainMenu.Items.Add(itemShowRulers);

        var itemSettings = new MenuItem { Header = "Settings" };
        itemSettings.Click += MenuItemSettings_Click;
        mainMenu.Items.Add(itemSettings);

        mainMenu.Items.Add(new Separator());

        var itemExit = new MenuItem { Header = "Exit" };
        itemExit.Click += MenuItemExit_Click;
        mainMenu.Items.Add(itemExit);

        FmMain.ContextMenu = mainMenu;
    }

    private void ItemEditClick(object sender, RoutedEventArgs e)
    {
        var fmEdit = new FmEdit();
        fmEdit.ShowDialog();
    }

    private void ItemMakeMainOnClick(object sender, RoutedEventArgs e)
    {
        int index = gdMain.Children.IndexOf(_lastRightClickedClock);
        if (_lastRightClickedClock != null)
        {
            App.Settings.LstClock[App.Settings.MainClockIndex].IsMain = false;
            App.Settings.MainClockIndex = index;
            App.Settings.LstClock[index].IsMain = true;
            App.Settings.MainTimeZone = App.Settings.LstClock[index].timeZone;
        }

        fmRulers.UpdateRulers();
    }

    private void MenuItemShowRulers_Click(object sender, RoutedEventArgs e)
    {
        if (fmSlider.Visibility == Visibility.Visible)
        {
            fmSlider.Hide();
            fmRulers.Hide();
            _timeTimer!.Interval = TimeSpan.FromSeconds(1);
        }
        else
        {
            var curTime = TimeZoneInfo.ConvertTimeBySystemTimeZoneId(_localTime, App.Settings.MainTimeZone);
            // Round to nearest hour
            if (curTime.Minute >= 30)
                curTime = curTime.AddHours(1);
            curTime = new DateTime(curTime.Year, curTime.Month, curTime.Day, curTime.Hour, 0, 0);
            fmSlider.CurTime = curTime;
            fmSlider.slTimeChecker.Value = curTime.Hour * 12; // + (int)(curTime.Minute / 5);
            fmSlider.Show();
            fmRulers.Show();
            // _timeTimer!.Interval = TimeSpan.FromMicroseconds(100);
        }

        fmMain_MouseLeave(null!, null!);
    }

    private void MenuItemFold_Click(object sender, RoutedEventArgs e)
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

    private void ItemMoveRightOnClick(object sender, RoutedEventArgs e)
    {
        throw new NotImplementedException();
    }

    private void ItemMoveLeftOnClick(object sender, RoutedEventArgs e)
    {
        throw new NotImplementedException();
    }

    private UIElement? _lastRightClickedClock;

    private void ContextMenu_Opened(object sender, RoutedEventArgs e)
    {
        Point pos = Mouse.GetPosition(gdMain); // relative to the grid
        HitTestResult result = VisualTreeHelper.HitTest(gdMain, pos);


        if (result != null)
        {
            // Traverse up the tree to find the OsClock/Grid you added
            DependencyObject current = result.VisualHit;
            while (current != null && !(current is Grid && gdMain.Children.Contains((UIElement)current)))
            {
                current = VisualTreeHelper.GetParent(current);
            }

            _lastRightClickedClock = current as UIElement;
            int index = gdMain.Children.IndexOf(_lastRightClickedClock);

            if (App.Settings.LstClock[index].IsMain)
            {
                itemMakeMain.Visibility = Visibility.Collapsed;
            }
            else
            {
                itemMakeMain.Visibility = Visibility.Visible;
            }

            itemMoveLeft.IsEnabled = (index == 0) ? false : true;
            itemMoveRight.IsEnabled = (index == App.Settings.LstClock.Count - 1) ? false : true;
            itemRemove.IsEnabled = (App.Settings.LstClock.Count == 1) ? false : true;
        }
    }

    private void MenuItemChangeTimeZone_Click(object sender, RoutedEventArgs e)
    {
        throw new NotImplementedException();
    }

    private void MenuItemRename_Click(object sender, RoutedEventArgs e)
    {
        throw new NotImplementedException();
    }


    private void MenuItemRemove_Click(object sender, RoutedEventArgs e)
    {
        if (_lastRightClickedClock != null)
        {
            // Prevent removing the last clock
            if (App.Settings.LstClock.Count <= 1)
            {
                MessageBox.Show("Cannot remove the last clock.", "Warning", MessageBoxButton.OK,
                    MessageBoxImage.Warning);
                return;
            }

            var clockToRemove = App.Settings.LstClock
                .FirstOrDefault(c => c.OsGrid == _lastRightClickedClock);
            if (clockToRemove.IsMain)
            {
                MessageBox.Show("Cannot remove Main clock.", "Warning", MessageBoxButton.OK,
                    MessageBoxImage.Warning);
                return;
            }

            if (MessageBox.Show("Are you sure you want to delete this clock?", "Confirm Deletion",
                    MessageBoxButton.YesNo, MessageBoxImage.Warning) != MessageBoxResult.Yes)
                return;

            if (clockToRemove != null)
            {
                if (fmRulers.IsLoaded)
                {
                    fmRulers.glRulers.Children.Remove(clockToRemove.RulerGrid);
                    fmRulers.InitializeRulers();
                }

                App.Settings.LstClock.Remove(clockToRemove);
                gdMain.Children.Remove(clockToRemove.OsGrid);

                fmSlider.Size -= 1;
            }

            _lastRightClickedClock = null;
        }
    }


    private void read_Config()
    {
        Left = App.Settings.MainWndLeft;
        Top = App.Settings.MainWndTop;
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
        var utcNow = DateTime.UtcNow;
        if (fmSlider.Visibility == Visibility.Visible)
        {
            var localOffset = TimeZoneInfo.Local.GetUtcOffset(utcNow);
            var targetOffset = TimeZoneInfo.FindSystemTimeZoneById(App.Settings.MainTimeZone).GetUtcOffset(utcNow);

            _localTime = fmSlider.CurTime.Date + (localOffset - targetOffset);

            _localTime = _localTime.AddMinutes((int)fmSlider.slTimeChecker.Value * 5);
            fmRulers.rwTop.Height = new GridLength(fmRulers.rwTop.MaxHeight *
                fmSlider.slTimeChecker.Value / fmSlider.slTimeChecker.Maximum);
        }
        else
        {
            _localTime = utcNow;
        }

        foreach (var item in App.Settings.LstClock!)
            item.SetTime(_localTime);

        ForceToTopmost();
    }

    private void MenuItemSettings_Click(object sender, RoutedEventArgs e)
    {
        var newFmSettings = new FmSettings(this);
        newFmSettings.ShowDialog();
    }

    private void AdjustTimeCheckerPosition()
    {
        fmRulers.Left = Left;
        fmRulers.Top = Top + Height;
        fmSlider.Left = Left;
        fmSlider.Top = Top + Height + fmRulers.Height;
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


    private void fmMain_MouseDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true; //double click  
        if (e.ClickCount > 1)
        {
            MenuItemShowRulers_Click(this, null!);
        }

        // fmSlider.Size = fmSlider.Size - 1;

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

    private void UpdateOpacity()
    {
        if (isMouseOver || isWindowFocused)
            AnimateOpacity(1.0); // Fully visible
        else
            AnimateOpacity(App.Settings.Opacity); // Use custom opacity
    }

    private void fmMain_MouseEnter(object sender, MouseEventArgs e)
    {
        isMouseOver = true;
        AnimateOpacity(1.0); // Fully visible
    }

    private void fmMain_MouseLeave(object sender, MouseEventArgs e)
    {
        isMouseOver = false;
        UpdateOpacity();
    }

    private void AnimateOpacity(double toOpacity)
    {
        var animation = new DoubleAnimation
        {
            To = toOpacity,
            Duration = TimeSpan.FromMilliseconds(300), // adjust speed as needed
            EasingFunction = new QuadraticEase { EasingMode = EasingMode.EaseInOut }
        };

        BeginAnimation(Window.OpacityProperty, animation);
    }

    private void FoldMainWindow()
    {
        AnimateWindowHeight(FoldedHeight);
        _isFolded = true;
    }

    private void UnFoldMainWindow()
    {
        AnimateWindowHeight(UnfoldedHeight);
        _isFolded = false;
    }

    private void AnimateWindowHeight(double toHeight)
    {
        var animation = new DoubleAnimation
        {
            To = toHeight,
            Duration = TimeSpan.FromMilliseconds(200),
            EasingFunction = new QuadraticEase { EasingMode = EasingMode.EaseInOut }
        };

        animation.Completed += (s, e) => AdjustTimeCheckerPosition();

        // Create a clock to track the animation frame-by-frame
        var animClock = animation.CreateClock();
        ApplyAnimationClock(HeightProperty, animClock);

        animClock.CurrentTimeInvalidated += (s, e) =>
        {
            // Each tick of the animation, update attached windows
            fmRulers.Left = Left;
            fmRulers.Top = Top + Height;

            fmSlider.Left = Left;
            fmSlider.Top = Top + Height + fmRulers.Height;
        };
    }

    private void ForceToTopmost()
    {
        // Get the window handle
        IntPtr windowHandle = new WindowInteropHelper(this).Handle;

        // Set window to topmost position without activating it
        SetWindowPos(windowHandle,
            (IntPtr)HWND_TOPMOST,
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }

    private void FmMainWindow_OnLostFocus(object sender, RoutedEventArgs e)
    {
        AnimateOpacity(App.Settings.Opacity);
    }
}
// _lstClock.Add(new OsClock("NYK", "Eastern Standard Time", "#FFAAAAFF", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("LDN", "GMT Standard Time", "#FFAAFFAA", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("KYIV", "FLE Standard Time", "#FFAAFFFF", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("PUN", "India Standard Time", "#FF99BBBB", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("SGP", "Singapore Standard Time", "#FFFFFFAA", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("TKO", "Tokyo Standard Time", "#FFFFAAAA", _lstClock.Count * 100));

//I have another two window which are attached (connected to bottom side) to the main window. Now  folding is applied to the main window and once it's finished, attached windows change their position. Is it possible to move attached